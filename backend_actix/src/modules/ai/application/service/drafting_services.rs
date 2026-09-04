//! Tailoring a CV, and drafting a cover letter.
//!
//! Both stream, and both share the same arrangement: gather the material,
//! spend one generation, hand back a stream. The only difference is the
//! standing instruction, which is why they are one service with two
//! implementations rather than two nearly identical ones.

use async_trait::async_trait;
use std::sync::Arc;

use crate::ai::application::ports::incoming::use_cases::{
    AiError, ConsumeAiQuotaUseCase, CoverLetterDraftUseCase, DraftingInput, TailorUseCase,
};
use crate::ai::application::ports::outgoing::{
    DraftingContextError, DraftingContextReader, DraftingMaterial, GenerationRequest,
    GenerationStream, TextGenerator,
};
use crate::auth::application::domain::entities::UserId;

impl From<DraftingContextError> for AiError {
    fn from(e: DraftingContextError) -> Self {
        match e {
            DraftingContextError::NotFound => AiError::Invalid("Application not found".to_string()),
            DraftingContextError::NoCv => AiError::Invalid(e.to_string()),
            DraftingContextError::Failed(m) => AiError::Upstream(m),
        }
    }
}

const TAILOR_SYSTEM: &str = "\
You help someone tailor their CV to one job. Suggest specific, checkable \
changes grounded in what the CV already says. Never invent experience, \
employers, dates or numbers the CV does not contain — a suggestion that puts \
something untrue in front of an employer is worse than no suggestion. Where \
the job asks for something the CV does not evidence, say so plainly instead of \
papering over it.";

const LETTER_SYSTEM: &str = "\
You draft cover letters. Write in the applicant's own register, grounded only \
in what their CV says. Never claim experience the CV does not contain. Prefer \
plain sentences over enthusiasm.";

/// Assembles the prompt.
///
/// Ordering is load-bearing: the CV and the posting are stable across a whole
/// working session and go in `context`, and only `instruction` changes between
/// turns. That separation is what lets the provider serve the expensive part
/// from cache — see the port's documentation.
fn request(
    system: &str,
    material: &DraftingMaterial,
    instruction: String,
    language: Option<String>,
    max_output_tokens: u32,
) -> GenerationRequest {
    let mut context = format!(
        "# The CV\n\n{}\n\n# The job\n\n{}",
        material.cv, material.job
    );

    if let Some(letter) = &material.existing_letter {
        if !letter.trim().is_empty() {
            context.push_str(&format!("\n\n# The letter so far\n\n{letter}"));
        }
    }

    // Appended to the instruction rather than the context: the language is a
    // per-turn choice, and putting it in the cached prefix would mean changing
    // it threw the cache away.
    let instruction = match language {
        Some(lang) if !lang.trim().is_empty() => {
            format!("{instruction}\n\nWrite in this language: {lang}.")
        }
        _ => instruction,
    };

    GenerationRequest {
        system: system.to_string(),
        context,
        instruction,
        max_output_tokens,
        // No schema: both of these produce prose for a person to read and
        // edit, and constraining them to JSON would only get in the way.
        schema: None,
    }
}

/// Implements both drafting contracts.
pub struct DraftingService<C> {
    quota: Arc<dyn ConsumeAiQuotaUseCase + Send + Sync>,
    generator: Option<Arc<dyn TextGenerator>>,
    context: C,
}

impl<C> DraftingService<C> {
    /// Builds it from the ports it depends on.
    pub fn new(
        quota: Arc<dyn ConsumeAiQuotaUseCase + Send + Sync>,
        generator: Option<Arc<dyn TextGenerator>>,
        context: C,
    ) -> Self {
        Self {
            quota,
            generator,
            context,
        }
    }

    /// The shared path: gather, spend, stream.
    async fn run(
        &self,
        owner: UserId,
        input: DraftingInput,
        system: &str,
        default_instruction: &str,
        max_output_tokens: u32,
    ) -> Result<GenerationStream, AiError>
    where
        C: DraftingContextReader,
    {
        let generator = self.generator.as_ref().ok_or(AiError::Disabled)?;

        let material = self
            .context
            .load(owner.value(), input.application_id, input.cv_id)
            .await?;

        // Spent before the stream opens, so an exhausted allowance refuses
        // cleanly rather than after a person has watched text start arriving.
        self.quota.execute(owner).await?;

        let instruction = input
            .instruction
            .filter(|i| !i.trim().is_empty())
            .unwrap_or_else(|| default_instruction.to_string());

        Ok(generator
            .generate_stream(request(
                system,
                &material,
                instruction,
                input.language,
                max_output_tokens,
            ))
            .await?)
    }
}

#[async_trait]
impl<C> TailorUseCase for DraftingService<C>
where
    C: DraftingContextReader + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        input: DraftingInput,
    ) -> Result<GenerationStream, AiError> {
        self.run(
            owner,
            input,
            TAILOR_SYSTEM,
            "Suggest how this CV could better answer this job.",
            8192,
        )
        .await
    }
}

#[async_trait]
impl<C> CoverLetterDraftUseCase for DraftingService<C>
where
    C: DraftingContextReader + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        input: DraftingInput,
    ) -> Result<GenerationStream, AiError> {
        self.run(
            owner,
            input,
            LETTER_SYSTEM,
            "Draft a cover letter for this application.",
            4096,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::application::ports::incoming::use_cases::QuotaError;
    use crate::ai::application::ports::outgoing::{Generation, GenerationError, GenerationEvent};
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

    #[derive(Default)]
    struct StubGenerator {
        seen: Mutex<Vec<GenerationRequest>>,
    }

    #[async_trait]
    impl TextGenerator for StubGenerator {
        fn provider(&self) -> &'static str {
            "stub"
        }
        async fn generate(&self, _r: GenerationRequest) -> Result<Generation, GenerationError> {
            unimplemented!()
        }
        async fn generate_stream(
            &self,
            request: GenerationRequest,
        ) -> Result<GenerationStream, GenerationError> {
            self.seen.lock().unwrap().push(request);
            Ok(Box::pin(futures::stream::iter(vec![Ok(
                GenerationEvent::Delta("drafted".into()),
            )])))
        }
    }

    struct StubContext(Result<DraftingMaterial, DraftingContextError>);

    #[async_trait]
    impl DraftingContextReader for StubContext {
        async fn load(
            &self,
            _o: Uuid,
            _a: Uuid,
            _c: Option<Uuid>,
        ) -> Result<DraftingMaterial, DraftingContextError> {
            self.0.clone()
        }
    }

    fn material() -> DraftingMaterial {
        DraftingMaterial {
            cv: "Jane Doe, Backend Engineer".into(),
            job: "Hiring a Senior Backend Engineer".into(),
            existing_letter: None,
        }
    }

    fn owner() -> UserId {
        UserId::from(Uuid::new_v4())
    }

    fn input() -> DraftingInput {
        DraftingInput {
            application_id: Uuid::new_v4(),
            ..Default::default()
        }
    }

    fn service(
        q: Arc<StubQuota>,
        g: Arc<StubGenerator>,
        ctx: StubContext,
    ) -> DraftingService<StubContext> {
        DraftingService::new(
            q as Arc<dyn ConsumeAiQuotaUseCase + Send + Sync>,
            Some(g as Arc<dyn TextGenerator>),
            ctx,
        )
    }

    /// The separation that makes caching possible: the CV and the posting are
    /// stable across a session and go in the cached part; only the instruction
    /// changes between turns. Merging them would throw the saving away on
    /// every vendor, silently.
    #[tokio::test]
    async fn the_cv_and_job_go_in_the_cacheable_context_not_the_instruction() {
        let gen = Arc::new(StubGenerator::default());
        let svc = service(quota(false), Arc::clone(&gen), StubContext(Ok(material())));

        let _stream = TailorUseCase::execute(&svc, owner(), input())
            .await
            .unwrap();

        let sent = &gen.seen.lock().unwrap()[0];
        assert!(sent.context.contains("Jane Doe"));
        assert!(sent.context.contains("Senior Backend Engineer"));
        assert!(
            !sent.instruction.contains("Jane Doe"),
            "the CV must not be in the volatile half"
        );
    }

    /// Language is a per-turn choice, so it rides with the instruction. In the
    /// cached prefix, changing it would discard the cache.
    #[tokio::test]
    async fn the_language_rides_with_the_instruction() {
        let gen = Arc::new(StubGenerator::default());
        let svc = service(quota(false), Arc::clone(&gen), StubContext(Ok(material())));

        let _stream = CoverLetterDraftUseCase::execute(
            &svc,
            owner(),
            DraftingInput {
                language: Some("id".into()),
                ..input()
            },
        )
        .await
        .unwrap();

        let sent = &gen.seen.lock().unwrap()[0];
        assert!(sent.instruction.contains("id"));
        assert!(!sent.context.contains("id\n"));
    }

    /// Refusing after a person has watched text begin to arrive is the worst
    /// moment to refuse, so the allowance is checked before the stream opens.
    #[tokio::test]
    async fn an_exhausted_allowance_refuses_before_the_stream_opens() {
        let gen = Arc::new(StubGenerator::default());
        let svc = service(quota(true), Arc::clone(&gen), StubContext(Ok(material())));

        let err = TailorUseCase::execute(&svc, owner(), input())
            .await
            .err()
            .unwrap();

        assert!(matches!(err, AiError::QuotaExceeded(_)));
        assert!(gen.seen.lock().unwrap().is_empty());
    }

    /// Gathering happens first, so an application that cannot be worked on
    /// costs nothing.
    #[tokio::test]
    async fn a_missing_application_spends_no_allowance() {
        let q = quota(false);
        let svc = service(
            Arc::clone(&q),
            Arc::new(StubGenerator::default()),
            StubContext(Err(DraftingContextError::NotFound)),
        );

        let _ = TailorUseCase::execute(&svc, owner(), input()).await;

        assert_eq!(*q.spent.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn a_draft_with_no_cv_says_what_to_send() {
        let svc = service(
            quota(false),
            Arc::new(StubGenerator::default()),
            StubContext(Err(DraftingContextError::NoCv)),
        );

        let err = CoverLetterDraftUseCase::execute(&svc, owner(), input())
            .await
            .err()
            .unwrap();

        assert!(matches!(err, AiError::Invalid(m) if m.contains("cv_id")));
    }

    #[tokio::test]
    async fn no_configured_provider_reports_disabled() {
        let svc = DraftingService::new(
            quota(false) as Arc<dyn ConsumeAiQuotaUseCase + Send + Sync>,
            None,
            StubContext(Ok(material())),
        );

        let err = TailorUseCase::execute(&svc, owner(), input())
            .await
            .err()
            .unwrap();

        assert!(matches!(err, AiError::Disabled));
    }

    /// The two surfaces are the same machinery with different standing
    /// instructions, and both instruct against inventing experience — the one
    /// failure that would put something untrue in front of an employer.
    #[tokio::test]
    async fn both_surfaces_forbid_inventing_experience() {
        let gen = Arc::new(StubGenerator::default());
        let svc = service(quota(false), Arc::clone(&gen), StubContext(Ok(material())));

        let _tailor = TailorUseCase::execute(&svc, owner(), input())
            .await
            .unwrap();
        let _letter = CoverLetterDraftUseCase::execute(&svc, owner(), input())
            .await
            .unwrap();

        for sent in gen.seen.lock().unwrap().iter() {
            assert!(
                sent.system.to_lowercase().contains("never invent")
                    || sent.system.to_lowercase().contains("never claim"),
                "system prompt must forbid invented experience: {}",
                sent.system
            );
        }
    }

    #[tokio::test]
    async fn an_existing_letter_is_given_to_the_model_to_revise() {
        let gen = Arc::new(StubGenerator::default());
        let svc = service(
            quota(false),
            Arc::clone(&gen),
            StubContext(Ok(DraftingMaterial {
                existing_letter: Some("Dear hiring manager".into()),
                ..material()
            })),
        );

        let _stream = CoverLetterDraftUseCase::execute(&svc, owner(), input())
            .await
            .unwrap();

        assert!(gen.seen.lock().unwrap()[0]
            .context
            .contains("Dear hiring manager"));
    }
}
