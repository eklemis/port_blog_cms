//! Reading a job posting into typed fields.

use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

use crate::ai::application::ports::incoming::use_cases::{
    AiError, ConsumeAiQuotaUseCase, ExtractJobInput, ExtractJobUseCase, ExtractedJob,
};
use crate::ai::application::ports::outgoing::{GenerationRequest, PostingFetcher, TextGenerator};
use crate::auth::application::domain::entities::UserId;

/// What a posting is turned into.
///
/// Constrained by schema so the result fills the capture form directly. Prose
/// that the frontend had to pattern-match would fail quietly and differently
/// every time; a malformed generation fails loudly instead.
fn schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "title":           { "type": "string" },
            "company":         { "type": "string" },
            "location":        { "type": "string" },
            "seniority":       { "type": "string" },
            "required_skills": { "type": "array", "items": { "type": "string" } },
            "nice_to_have":    { "type": "array", "items": { "type": "string" } }
        },
        "required": [
            "title", "company", "location", "seniority",
            "required_skills", "nice_to_have"
        ],
        "additionalProperties": false
    })
}

const SYSTEM: &str = "\
You read job postings and return their contents as structured fields. Copy \
what the posting says; do not infer, improve or invent. When the posting does \
not state something, return an empty string or an empty list for it rather \
than guessing.";

/// Implements the corresponding use-case contract.
pub struct ExtractJobService<F> {
    quota: Arc<dyn ConsumeAiQuotaUseCase + Send + Sync>,
    generator: Option<Arc<dyn TextGenerator>>,
    fetcher: F,
}

impl<F> ExtractJobService<F> {
    /// Builds it from the ports it depends on.
    ///
    /// `generator` is optional because a deployment without credentials runs
    /// with generation switched off rather than failing to start.
    pub fn new(
        quota: Arc<dyn ConsumeAiQuotaUseCase + Send + Sync>,
        generator: Option<Arc<dyn TextGenerator>>,
        fetcher: F,
    ) -> Self {
        Self {
            quota,
            generator,
            fetcher,
        }
    }
}

#[async_trait]
impl<F> ExtractJobUseCase for ExtractJobService<F>
where
    F: PostingFetcher + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        input: ExtractJobInput,
    ) -> Result<ExtractedJob, AiError> {
        let generator = self.generator.as_ref().ok_or(AiError::Disabled)?;

        // Pasted text wins over a URL when both are sent: it is the thing the
        // person is actually looking at, and it cannot fail to load.
        let posting = match (input.text, input.url) {
            (Some(text), _) if !text.trim().is_empty() => text,
            (_, Some(url)) if !url.trim().is_empty() => {
                // One attempt, no retry. Most boards block this, the failure is
                // expected, and the caller is one paste away from succeeding —
                // making them wait through retries helps nobody.
                self.fetcher
                    .fetch(&url)
                    .await
                    .map_err(AiError::FetchFailed)?
            }
            _ => {
                return Err(AiError::Invalid(
                    "Send the posting as text, or a url to fetch it from".into(),
                ))
            }
        };

        // Spent before the provider is called, so an exhausted allowance
        // refuses without costing anything. A generation that is then refused
        // or fails still counts: the provider did the work and billed for it.
        self.quota.execute(owner).await?;

        let generation = generator
            .generate(GenerationRequest {
                system: SYSTEM.to_string(),
                context: posting,
                instruction: "Return this posting's fields.".to_string(),
                max_output_tokens: 2048,
                schema: Some(schema()),
            })
            .await?;

        serde_json::from_str(&generation.text).map_err(|e| {
            AiError::Upstream(format!("the model's reply did not match the schema: {e}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::application::ports::incoming::use_cases::QuotaError;
    use crate::ai::application::ports::outgoing::{
        Generation, GenerationError, GenerationStream, Usage,
    };
    use crate::ai::domain::quota::QuotaState;
    use std::sync::Mutex;
    use uuid::Uuid;

    struct StubQuota {
        exhausted: bool,
        spent: Mutex<u32>,
    }

    #[async_trait]
    impl ConsumeAiQuotaUseCase for StubQuota {
        async fn execute(&self, _o: UserId) -> Result<QuotaState, QuotaError> {
            if self.exhausted {
                return Err(QuotaError::Exceeded(Box::new(QuotaState {
                    used: 5,
                    limit: Some(5),
                    resets_at: chrono::Utc::now(),
                })));
            }
            *self.spent.lock().unwrap() += 1;
            Ok(QuotaState {
                used: 1,
                limit: None,
                resets_at: chrono::Utc::now(),
            })
        }
    }

    fn quota(exhausted: bool) -> Arc<StubQuota> {
        Arc::new(StubQuota {
            exhausted,
            spent: Mutex::new(0),
        })
    }

    struct StubGenerator {
        reply: String,
        seen: Mutex<Vec<GenerationRequest>>,
    }

    #[async_trait]
    impl TextGenerator for StubGenerator {
        fn provider(&self) -> &'static str {
            "stub"
        }
        async fn generate(
            &self,
            request: GenerationRequest,
        ) -> Result<Generation, GenerationError> {
            self.seen.lock().unwrap().push(request);
            Ok(Generation {
                text: self.reply.clone(),
                usage: Usage::default(),
            })
        }
        async fn generate_stream(
            &self,
            _r: GenerationRequest,
        ) -> Result<GenerationStream, GenerationError> {
            unimplemented!()
        }
    }

    fn generator(reply: &str) -> Arc<StubGenerator> {
        Arc::new(StubGenerator {
            reply: reply.to_string(),
            seen: Mutex::new(vec![]),
        })
    }

    struct StubFetcher(Result<String, String>);

    #[async_trait]
    impl PostingFetcher for StubFetcher {
        async fn fetch(&self, _url: &str) -> Result<String, String> {
            self.0.clone()
        }
    }

    fn owner() -> UserId {
        UserId::from(Uuid::new_v4())
    }

    const GOOD_REPLY: &str = r#"{"title":"Backend Engineer","company":"Acme",
        "location":"Remote","seniority":"Senior",
        "required_skills":["Rust"],"nice_to_have":[]}"#;

    #[tokio::test]
    async fn pasted_text_is_read_into_fields() {
        let service = ExtractJobService::new(
            quota(false),
            Some(generator(GOOD_REPLY) as Arc<dyn TextGenerator>),
            StubFetcher(Err("unused".into())),
        );

        let job = service
            .execute(
                owner(),
                ExtractJobInput {
                    text: Some("We are hiring a Senior Backend Engineer".into()),
                    url: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(job.title, "Backend Engineer");
        assert_eq!(job.required_skills, vec!["Rust"]);
    }

    /// Pasted text is what the person is looking at, and it cannot fail to
    /// load. A URL sent alongside it is a shortcut, not an override.
    #[tokio::test]
    async fn pasted_text_wins_over_a_url() {
        let gen = generator(GOOD_REPLY);
        let service = ExtractJobService::new(
            quota(false),
            Some(Arc::clone(&gen) as Arc<dyn TextGenerator>),
            StubFetcher(Ok("fetched instead".into())),
        );

        service
            .execute(
                owner(),
                ExtractJobInput {
                    text: Some("pasted".into()),
                    url: Some("https://example.com/job".into()),
                },
            )
            .await
            .unwrap();

        assert_eq!(gen.seen.lock().unwrap()[0].context, "pasted");
    }

    /// Expected, not exceptional. Most boards block automated fetches, so this
    /// answers specifically rather than retrying — the caller is one paste
    /// away from succeeding.
    #[tokio::test]
    async fn a_failed_fetch_says_so_specifically() {
        let service = ExtractJobService::new(
            quota(false),
            Some(generator(GOOD_REPLY) as Arc<dyn TextGenerator>),
            StubFetcher(Err("403 Forbidden".into())),
        );

        let err = service
            .execute(
                owner(),
                ExtractJobInput {
                    text: None,
                    url: Some("https://example.com/job".into()),
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, AiError::FetchFailed(m) if m.contains("403")));
    }

    /// A fetch that never happened must not cost anything.
    #[tokio::test]
    async fn a_failed_fetch_spends_no_allowance() {
        let q = quota(false);
        let service = ExtractJobService::new(
            Arc::clone(&q) as Arc<dyn ConsumeAiQuotaUseCase + Send + Sync>,
            Some(generator(GOOD_REPLY) as Arc<dyn TextGenerator>),
            StubFetcher(Err("403".into())),
        );

        let _ = service
            .execute(
                owner(),
                ExtractJobInput {
                    text: None,
                    url: Some("https://example.com/job".into()),
                },
            )
            .await;

        assert_eq!(*q.spent.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn an_exhausted_allowance_refuses_before_calling_the_provider() {
        let gen = generator(GOOD_REPLY);
        let service = ExtractJobService::new(
            quota(true),
            Some(Arc::clone(&gen) as Arc<dyn TextGenerator>),
            StubFetcher(Err("unused".into())),
        );

        let err = service
            .execute(
                owner(),
                ExtractJobInput {
                    text: Some("posting".into()),
                    url: None,
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, AiError::QuotaExceeded(_)));
        assert!(
            gen.seen.lock().unwrap().is_empty(),
            "the provider must not be called once the allowance is spent"
        );
    }

    /// A deployment without credentials runs with generation off rather than
    /// failing to start, so the routes have to say so.
    #[tokio::test]
    async fn no_configured_provider_reports_disabled() {
        let service: ExtractJobService<StubFetcher> =
            ExtractJobService::new(quota(false), None, StubFetcher(Err("unused".into())));

        let err = service
            .execute(
                owner(),
                ExtractJobInput {
                    text: Some("posting".into()),
                    url: None,
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, AiError::Disabled));
    }

    #[tokio::test]
    async fn neither_text_nor_url_is_a_bad_request() {
        let service = ExtractJobService::new(
            quota(false),
            Some(generator(GOOD_REPLY) as Arc<dyn TextGenerator>),
            StubFetcher(Err("unused".into())),
        );

        let err = service
            .execute(owner(), ExtractJobInput::default())
            .await
            .unwrap_err();

        assert!(matches!(err, AiError::Invalid(_)));
    }

    /// The schema is what makes this fill a form. A reply that does not match
    /// it must fail loudly rather than half-populating the capture screen.
    #[tokio::test]
    async fn a_reply_that_ignores_the_schema_fails_loudly() {
        let service = ExtractJobService::new(
            quota(false),
            Some(
                generator("Sure! Here is the job: Backend Engineer at Acme.")
                    as Arc<dyn TextGenerator>,
            ),
            StubFetcher(Err("unused".into())),
        );

        let err = service
            .execute(
                owner(),
                ExtractJobInput {
                    text: Some("posting".into()),
                    url: None,
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, AiError::Upstream(_)));
    }

    #[tokio::test]
    async fn the_schema_is_sent_so_the_reply_is_constrained() {
        let gen = generator(GOOD_REPLY);
        let service = ExtractJobService::new(
            quota(false),
            Some(Arc::clone(&gen) as Arc<dyn TextGenerator>),
            StubFetcher(Err("unused".into())),
        );

        service
            .execute(
                owner(),
                ExtractJobInput {
                    text: Some("posting".into()),
                    url: None,
                },
            )
            .await
            .unwrap();

        let sent = &gen.seen.lock().unwrap()[0];
        assert!(
            sent.schema.is_some(),
            "extraction must constrain its output"
        );
    }
}
