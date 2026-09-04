//! Supplies `career` with a model's judgement, without `career` knowing a
//! model is what answers.
//!
//! Asks for **verdicts and evidence, never a score**. The score is arithmetic
//! over the verdicts and is computed in `career`, so the model cannot hand
//! back a number that disagrees with its own rows.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::ai::application::ports::incoming::use_cases::{ConsumeAiQuotaUseCase, QuotaError};
use crate::ai::application::ports::outgoing::{GenerationError, GenerationRequest, TextGenerator};
use crate::auth::application::domain::entities::UserId;
use crate::career::application::ports::outgoing::{RelevanceEstimator, RelevanceEstimatorError};
use crate::career::domain::relevance::{RequirementMatch, Verdict};

const SYSTEM: &str = "\
You compare a CV against one job posting. List every requirement the posting \
states, in its own words and in its own order. For each, judge whether the CV \
evidences it (met), gestures at it without evidencing it (partial), or does \
not address it (missing).

Quote the CV line you judged against as the evidence, verbatim. If nothing in \
the CV addresses a requirement, leave the evidence out rather than reaching \
for something loosely related — a quote that does not support the verdict is \
worse than none, because a reader will trust it.

Do not score anything. Do not suggest changes. Judge only what is written.";

/// The shape asked of the model.
///
/// No score field, deliberately — see the module documentation.
fn schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "requirements": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "text":     { "type": "string" },
                        "verdict":  { "type": "string", "enum": ["met", "partial", "missing"] },
                        "evidence": { "type": ["string", "null"] }
                    },
                    "required": ["text", "verdict", "evidence"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["requirements"],
        "additionalProperties": false
    })
}

#[derive(Debug, Deserialize)]
struct Reply {
    requirements: Vec<Row>,
}

#[derive(Debug, Deserialize)]
struct Row {
    text: String,
    verdict: Verdict,
    evidence: Option<String>,
}

/// Estimates relevance with a model.
pub struct RelevanceEstimatorAi {
    quota: Arc<dyn ConsumeAiQuotaUseCase + Send + Sync>,
    generator: Arc<dyn TextGenerator>,
}

impl RelevanceEstimatorAi {
    /// Builds it from the ports it depends on.
    pub fn new(
        quota: Arc<dyn ConsumeAiQuotaUseCase + Send + Sync>,
        generator: Arc<dyn TextGenerator>,
    ) -> Self {
        Self { quota, generator }
    }
}

/// Drops rows whose evidence does not appear in the CV.
///
/// A quoted line that is not actually in the document is the one failure that
/// would make this worse than useless: the evidence is what a reader checks,
/// and one invented quote discredits every real one. Rather than drop the row,
/// the quote is removed and the verdict kept — the judgement may still be
/// sound, and a missing quote is honest where a wrong one is not.
fn drop_unquotable_evidence(cv: &str, rows: Vec<Row>) -> Vec<RequirementMatch> {
    rows.into_iter()
        .map(|r| {
            let evidence = r.evidence.filter(|quote| {
                let quote = quote.trim();
                !quote.is_empty() && cv.contains(quote)
            });

            RequirementMatch {
                text: r.text,
                verdict: r.verdict,
                evidence,
            }
        })
        .collect()
}

#[async_trait]
impl RelevanceEstimator for RelevanceEstimatorAi {
    async fn estimate(
        &self,
        owner: Uuid,
        cv: &str,
        job: &str,
    ) -> Result<Vec<RequirementMatch>, RelevanceEstimatorError> {
        self.quota
            .execute(UserId::from(owner))
            .await
            .map_err(|e| match e {
                QuotaError::Exceeded(_) => RelevanceEstimatorError::QuotaExceeded,
                QuotaError::Unavailable(m) => RelevanceEstimatorError::Failed(m),
            })?;

        let generation = self
            .generator
            .generate(GenerationRequest {
                system: SYSTEM.to_string(),
                // The CV and posting are the stable half, so a second analysis
                // of the same pair is served from the provider's cache.
                context: format!("# The CV\n\n{cv}\n\n# The job\n\n{job}"),
                instruction: "List the posting's requirements and judge each against the CV."
                    .to_string(),
                max_output_tokens: 4096,
                schema: Some(schema()),
            })
            .await
            .map_err(|e| match e {
                GenerationError::Refused(_) => RelevanceEstimatorError::Refused,
                other => RelevanceEstimatorError::Failed(other.to_string()),
            })?;

        let reply: Reply = serde_json::from_str(&generation.text).map_err(|e| {
            RelevanceEstimatorError::Failed(format!("reply did not match the schema: {e}"))
        })?;

        Ok(drop_unquotable_evidence(cv, reply.requirements))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::application::ports::outgoing::{Generation, GenerationStream, Usage};
    use crate::ai::domain::quota::QuotaState;
    use std::sync::Mutex;

    struct StubQuota(bool);

    #[async_trait]
    impl ConsumeAiQuotaUseCase for StubQuota {
        async fn execute(&self, _o: UserId) -> Result<QuotaState, QuotaError> {
            if self.0 {
                return Err(QuotaError::Exceeded(Box::new(QuotaState {
                    used: 5,
                    limit: Some(5),
                    resets_at: chrono::Utc::now(),
                })));
            }
            Ok(QuotaState {
                used: 1,
                limit: None,
                resets_at: chrono::Utc::now(),
            })
        }
    }

    struct StubGen {
        reply: Result<String, GenerationError>,
        seen: Mutex<Vec<GenerationRequest>>,
    }

    #[async_trait]
    impl TextGenerator for StubGen {
        fn provider(&self) -> &'static str {
            "stub"
        }
        async fn generate(
            &self,
            request: GenerationRequest,
        ) -> Result<Generation, GenerationError> {
            self.seen.lock().unwrap().push(request);
            match &self.reply {
                Ok(text) => Ok(Generation {
                    text: text.clone(),
                    usage: Usage::default(),
                }),
                Err(e) => Err(e.clone()),
            }
        }
        async fn generate_stream(
            &self,
            _r: GenerationRequest,
        ) -> Result<GenerationStream, GenerationError> {
            unimplemented!()
        }
    }

    fn estimator(exhausted: bool, reply: Result<String, GenerationError>) -> RelevanceEstimatorAi {
        RelevanceEstimatorAi::new(
            Arc::new(StubQuota(exhausted)),
            Arc::new(StubGen {
                reply,
                seen: Mutex::new(vec![]),
            }),
        )
    }

    const CV: &str = "Jane Doe\nBuilt the order-events pipeline on Kafka";

    #[tokio::test]
    async fn verdicts_and_evidence_come_back() {
        let reply = r#"{"requirements":[
            {"text":"Kafka in production","verdict":"met",
             "evidence":"Built the order-events pipeline on Kafka"},
            {"text":"Kubernetes","verdict":"missing","evidence":null}
        ]}"#;

        let rows = estimator(false, Ok(reply.into()))
            .estimate(Uuid::new_v4(), CV, "a posting")
            .await
            .unwrap();

        assert_eq!(rows[0].verdict, Verdict::Met);
        assert_eq!(
            rows[0].evidence.as_deref(),
            Some("Built the order-events pipeline on Kafka")
        );
        assert_eq!(rows[1].verdict, Verdict::Missing);
        assert!(rows[1].evidence.is_none());
    }

    /// The failure that would make this worse than useless. Evidence is what a
    /// reader checks; one invented quote discredits every real one. The verdict
    /// survives — it may still be sound — but the quote does not.
    #[tokio::test]
    async fn evidence_that_is_not_in_the_cv_is_dropped() {
        let reply = r#"{"requirements":[
            {"text":"Kafka","verdict":"met","evidence":"Led a team of twelve"}
        ]}"#;

        let rows = estimator(false, Ok(reply.into()))
            .estimate(Uuid::new_v4(), CV, "a posting")
            .await
            .unwrap();

        assert_eq!(rows[0].verdict, Verdict::Met, "the judgement is kept");
        assert!(
            rows[0].evidence.is_none(),
            "a quote that is not in the CV must not be shown"
        );
    }

    #[tokio::test]
    async fn empty_evidence_is_treated_as_none() {
        let reply = r#"{"requirements":[
            {"text":"Kafka","verdict":"partial","evidence":"   "}
        ]}"#;

        let rows = estimator(false, Ok(reply.into()))
            .estimate(Uuid::new_v4(), CV, "a posting")
            .await
            .unwrap();

        assert!(rows[0].evidence.is_none());
    }

    /// The model is never asked for a score, so it cannot return one that
    /// disagrees with its own rows.
    #[tokio::test]
    async fn the_schema_asks_for_no_score() {
        let est = estimator(false, Ok(r#"{"requirements":[]}"#.into()));

        est.estimate(Uuid::new_v4(), CV, "a posting").await.unwrap();

        let schema = schema();
        let props = &schema["properties"]["requirements"]["items"]["properties"];
        assert!(props["score"].is_null(), "no score field may be requested");
        assert!(props["verdict"].is_object());
    }

    #[tokio::test]
    async fn an_exhausted_allowance_is_reported_as_such() {
        let err = estimator(true, Ok("unused".into()))
            .estimate(Uuid::new_v4(), CV, "a posting")
            .await
            .unwrap_err();

        assert!(matches!(err, RelevanceEstimatorError::QuotaExceeded));
    }

    #[tokio::test]
    async fn a_refusal_is_reported_as_such() {
        let err = estimator(false, Err(GenerationError::Refused("cyber".into())))
            .estimate(Uuid::new_v4(), CV, "a posting")
            .await
            .unwrap_err();

        assert!(matches!(err, RelevanceEstimatorError::Refused));
    }

    #[tokio::test]
    async fn a_reply_that_ignores_the_schema_fails_rather_than_half_parsing() {
        let err = estimator(false, Ok("Here are the requirements: Kafka.".into()))
            .estimate(Uuid::new_v4(), CV, "a posting")
            .await
            .unwrap_err();

        assert!(matches!(err, RelevanceEstimatorError::Failed(_)));
    }
}
