//! Producing a match analysis.
//!
//! Two halves, reported separately and never averaged: one measured here, one
//! estimated by a model. The measured half always runs. The estimated half is
//! best-effort — a provider that is unreachable, unwilling or out of allowance
//! costs the reader that half and nothing else, with a code saying which.

use async_trait::async_trait;
use chrono::{Datelike, Utc};
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::career::application::ports::incoming::use_cases::{
    AnalyseApplicationInput, AnalyseApplicationUseCase, AnalysisError, MatchAnalysis,
};
use crate::career::application::ports::outgoing::{
    ApplicationStore, CvReader, RelevanceEstimator, RelevanceEstimatorError,
};
use crate::career::domain::readability;
use crate::career::domain::relevance::RelevanceReport;
use crate::cv::domain::entities::CVInfo;

/// Implements the corresponding use-case contract.
pub struct AnalyseApplicationService<A, J, C> {
    applications: A,
    jobs: J,
    cvs: C,
    relevance: Option<std::sync::Arc<dyn RelevanceEstimator>>,
}

impl<A, J, C> AnalyseApplicationService<A, J, C> {
    /// Builds it from the ports it depends on.
    ///
    /// The estimator is optional: a deployment with no model configured still
    /// serves the measured half rather than failing the endpoint.
    pub fn new(
        applications: A,
        jobs: J,
        cvs: C,
        relevance: Option<std::sync::Arc<dyn RelevanceEstimator>>,
    ) -> Self {
        Self {
            applications,
            jobs,
            cvs,
            relevance,
        }
    }
}

#[async_trait]
impl<A, J, C> AnalyseApplicationUseCase for AnalyseApplicationService<A, J, C>
where
    A: ApplicationStore + Send + Sync,
    J: crate::career::application::ports::outgoing::JobStore + Send + Sync,
    C: CvReader + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        application_id: Uuid,
        input: AnalyseApplicationInput,
    ) -> Result<MatchAnalysis, AnalysisError> {
        let application = self
            .applications
            .find(owner.value(), application_id)
            .await
            .map_err(|e| AnalysisError::RepositoryError(e.to_string()))?
            .ok_or(AnalysisError::ApplicationNotFound)?;

        let cv: CVInfo = match (input.cv_id, application.cv_snapshot_id) {
            // A named CV wins over the snapshot even when both exist: naming
            // one is what tailoring does, and the point of tailoring is to
            // look at the version being worked on.
            (Some(cv_id), _) => self
                .cvs
                .read_cv(owner.value(), cv_id)
                .await
                .map_err(|e| AnalysisError::RepositoryError(e.to_string()))?
                .ok_or(AnalysisError::CvNotFound)?,

            // Already sent: the only honest thing to analyse is what went out.
            (None, Some(snapshot_id)) => self
                .cvs
                .read_snapshot(owner.value(), snapshot_id)
                .await
                .map_err(|e| AnalysisError::RepositoryError(e.to_string()))?
                .ok_or(AnalysisError::CvNotFound)?,

            (None, None) => return Err(AnalysisError::NoCvToAnalyse),
        };

        let readability = readability::assess(&cv, Utc::now().year());

        // The estimated half is best-effort. A model that is unreachable, out
        // of allowance or unwilling must not cost the reader the half that
        // was computed correctly — so this degrades to a reason rather than
        // failing the request.
        let (relevance, relevance_unavailable) =
            self.estimate(owner, &cv, application.job_id).await;

        Ok(MatchAnalysis {
            readability,
            relevance,
            relevance_unavailable,
        })
    }
}

impl<A, J, C> AnalyseApplicationService<A, J, C>
where
    A: ApplicationStore + Send + Sync,
    J: crate::career::application::ports::outgoing::JobStore + Send + Sync,
    C: CvReader + Send + Sync,
{
    /// Runs the model half, returning either a report or the reason there is
    /// not one.
    async fn estimate(
        &self,
        owner: UserId,
        cv: &CVInfo,
        job_id: Uuid,
    ) -> (Option<RelevanceReport>, Option<String>) {
        let Some(estimator) = &self.relevance else {
            return (None, Some("AI_DISABLED".to_string()));
        };

        let job = match self.jobs.find(owner.value(), job_id).await {
            Ok(Some(job)) => job,
            // The application pointed at a posting that is gone. The measured
            // half still stands, so this is a missing estimate rather than a
            // failed analysis.
            Ok(None) => return (None, Some("JOB_NOT_FOUND".to_string())),
            Err(e) => {
                tracing::warn!("Could not read the job for an analysis: {}", e);
                return (None, Some("AI_UPSTREAM_ERROR".to_string()));
            }
        };

        let job_text = if job.source_text.trim().is_empty() {
            format!(
                "{} at {}\nRequires: {}",
                job.title,
                job.company,
                job.required_skills.join(", ")
            )
        } else {
            job.source_text
        };

        match estimator
            .estimate(owner.value(), &render_cv(cv), &job_text)
            .await
        {
            Ok(requirements) => (Some(RelevanceReport::from_requirements(requirements)), None),
            Err(e) => {
                let code = match e {
                    RelevanceEstimatorError::Disabled => "AI_DISABLED",
                    RelevanceEstimatorError::QuotaExceeded => "AI_QUOTA_EXCEEDED",
                    RelevanceEstimatorError::Refused => "AI_REFUSED",
                    RelevanceEstimatorError::Failed(ref detail) => {
                        tracing::warn!("Relevance estimate failed: {}", detail);
                        "AI_UPSTREAM_ERROR"
                    }
                };
                (None, Some(code.to_string()))
            }
        }
    }
}

/// The CV as prose for the model.
///
/// Deliberately plain: field names in the input invite field names in the
/// output, and what is wanted back is evidence quoted from the text.
fn render_cv(cv: &CVInfo) -> String {
    let mut out = format!("{}\n{}\n\n{}", cv.display_name, cv.role, cv.bio);

    for s in &cv.core_skills {
        out.push_str(&format!("\nSkill: {} — {}", s.title, s.description));
    }
    for e in &cv.experiences {
        out.push_str(&format!(
            "\n{} at {} ({} to {}): {}",
            e.position,
            e.company,
            e.start_date,
            e.end_date.as_deref().unwrap_or("present"),
            e.description
        ));
        for line in e.tasks.iter().chain(e.achievements.iter()) {
            out.push_str(&format!("\n  - {line}"));
        }
    }
    for e in &cv.educations {
        out.push_str(&format!(
            "\n{}, {} ({})",
            e.degree, e.institution, e.graduation_year
        ));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::career::application::ports::outgoing::{
        ApplicationStoreError, CreateApplicationData, CvReaderError, PatchApplicationData,
    };
    use crate::career::domain::entities::{Application, ApplicationStatus};
    use crate::cv::domain::entities::{CoreSkill, Education, Experience};
    use std::sync::{Arc, Mutex};

    fn a_cv(role: &str) -> CVInfo {
        CVInfo {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            role: role.into(),
            display_name: "Jane".into(),
            bio: "Rust".into(),
            photo_url: String::new(),
            core_skills: vec![CoreSkill {
                title: "Rust".into(),
                description: "Five years".into(),
            }],
            educations: vec![Education {
                degree: "BSc".into(),
                institution: "Uni".into(),
                graduation_year: 2018,
            }],
            experiences: vec![Experience {
                company: "Acme".into(),
                position: "Engineer".into(),
                location: "Remote".into(),
                start_date: "2020-01".into(),
                end_date: None,
                description: "Built things".into(),
                tasks: vec![],
                achievements: vec![],
            }],
            highlighted_projects: vec![],
            contact_info: vec![],
        }
    }

    fn an_application(snapshot: Option<Uuid>) -> Application {
        Application {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            job_id: Uuid::new_v4(),
            cv_snapshot_id: snapshot,
            status: ApplicationStatus::Draft,
            applied_at: None,
            next_action: String::new(),
            next_action_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    struct FakeJobs(Option<crate::career::domain::entities::Job>);

    #[async_trait]
    impl crate::career::application::ports::outgoing::JobStore for FakeJobs {
        async fn create(
            &self,
            _o: Uuid,
            _d: crate::career::application::ports::outgoing::CreateJobData,
        ) -> Result<
            crate::career::domain::entities::Job,
            crate::career::application::ports::outgoing::JobStoreError,
        > {
            unimplemented!()
        }
        async fn list(
            &self,
            _o: Uuid,
        ) -> Result<
            Vec<crate::career::domain::entities::Job>,
            crate::career::application::ports::outgoing::JobStoreError,
        > {
            unimplemented!()
        }
        async fn find(
            &self,
            _o: Uuid,
            _id: Uuid,
        ) -> Result<
            Option<crate::career::domain::entities::Job>,
            crate::career::application::ports::outgoing::JobStoreError,
        > {
            Ok(self.0.clone())
        }
        async fn patch(
            &self,
            _o: Uuid,
            _id: Uuid,
            _d: crate::career::application::ports::outgoing::PatchJobData,
        ) -> Result<
            crate::career::domain::entities::Job,
            crate::career::application::ports::outgoing::JobStoreError,
        > {
            unimplemented!()
        }
        async fn archive(
            &self,
            _o: Uuid,
            _id: Uuid,
        ) -> Result<(), crate::career::application::ports::outgoing::JobStoreError> {
            unimplemented!()
        }
    }

    fn a_job() -> crate::career::domain::entities::Job {
        crate::career::domain::entities::Job {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            title: "Backend Engineer".into(),
            company: "Acme".into(),
            location: String::new(),
            seniority: String::new(),
            required_skills: vec!["Kafka".into()],
            nice_to_have: vec![],
            source_url: String::new(),
            source_text: "We need Kafka in production".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    struct FakeEstimator(
        Result<
            Vec<crate::career::domain::relevance::RequirementMatch>,
            crate::career::application::ports::outgoing::RelevanceEstimatorError,
        >,
    );

    #[async_trait]
    impl RelevanceEstimator for FakeEstimator {
        async fn estimate(
            &self,
            _o: Uuid,
            _cv: &str,
            _job: &str,
        ) -> Result<Vec<crate::career::domain::relevance::RequirementMatch>, RelevanceEstimatorError>
        {
            match &self.0 {
                Ok(v) => Ok(v.clone()),
                Err(e) => Err(match e {
                    RelevanceEstimatorError::Disabled => RelevanceEstimatorError::Disabled,
                    RelevanceEstimatorError::QuotaExceeded => {
                        RelevanceEstimatorError::QuotaExceeded
                    }
                    RelevanceEstimatorError::Refused => RelevanceEstimatorError::Refused,
                    RelevanceEstimatorError::Failed(m) => {
                        RelevanceEstimatorError::Failed(m.clone())
                    }
                }),
            }
        }
    }

    struct FakeApplications(Option<Application>);

    #[async_trait]
    impl ApplicationStore for FakeApplications {
        async fn create(
            &self,
            _o: Uuid,
            _d: CreateApplicationData,
        ) -> Result<Application, ApplicationStoreError> {
            unimplemented!()
        }
        async fn list(&self, _o: Uuid) -> Result<Vec<Application>, ApplicationStoreError> {
            unimplemented!()
        }
        async fn find(
            &self,
            _o: Uuid,
            _id: Uuid,
        ) -> Result<Option<Application>, ApplicationStoreError> {
            Ok(self.0.clone())
        }
        async fn patch(
            &self,
            _o: Uuid,
            _id: Uuid,
            _d: PatchApplicationData,
        ) -> Result<Application, ApplicationStoreError> {
            unimplemented!()
        }
        async fn archive(&self, _o: Uuid, _id: Uuid) -> Result<(), ApplicationStoreError> {
            unimplemented!()
        }
    }

    #[derive(Default)]
    struct FakeCvs {
        living: Option<CVInfo>,
        frozen: Option<CVInfo>,
        read: Mutex<Vec<&'static str>>,
    }

    #[async_trait]
    impl CvReader for Arc<FakeCvs> {
        async fn read_cv(&self, _o: Uuid, _id: Uuid) -> Result<Option<CVInfo>, CvReaderError> {
            self.read.lock().unwrap().push("living");
            Ok(self.living.clone())
        }
        async fn read_snapshot(
            &self,
            _o: Uuid,
            _id: Uuid,
        ) -> Result<Option<CVInfo>, CvReaderError> {
            self.read.lock().unwrap().push("frozen");
            Ok(self.frozen.clone())
        }
    }

    fn owner() -> UserId {
        UserId::from(Uuid::new_v4())
    }

    /// `null`, never zero. A client rendering a zero would tell someone their
    /// CV matched nothing, when in fact nothing was measured.
    #[tokio::test]
    async fn an_absent_estimate_is_null_rather_than_a_score_of_zero() {
        let cvs = Arc::new(FakeCvs {
            living: Some(a_cv("Backend Engineer")),
            ..Default::default()
        });

        let analysis = AnalyseApplicationService::new(
            FakeApplications(Some(an_application(None))),
            FakeJobs(Some(a_job())),
            cvs,
            None,
        )
        .execute(
            owner(),
            Uuid::new_v4(),
            AnalyseApplicationInput {
                cv_id: Some(Uuid::new_v4()),
            },
        )
        .await
        .unwrap();

        assert!(analysis.relevance.is_none());
        assert!(analysis.readability.score > 0);
    }

    /// Tailoring looks at the version being worked on, so a named CV wins
    /// over the snapshot even when the application already carries one.
    #[tokio::test]
    async fn a_named_cv_is_preferred_over_the_snapshot() {
        let cvs = Arc::new(FakeCvs {
            living: Some(a_cv("Living")),
            frozen: Some(a_cv("Frozen")),
            ..Default::default()
        });

        AnalyseApplicationService::new(
            FakeApplications(Some(an_application(Some(Uuid::new_v4())))),
            FakeJobs(Some(a_job())),
            Arc::clone(&cvs),
            None,
        )
        .execute(
            owner(),
            Uuid::new_v4(),
            AnalyseApplicationInput {
                cv_id: Some(Uuid::new_v4()),
            },
        )
        .await
        .unwrap();

        assert_eq!(cvs.read.lock().unwrap().as_slice(), &["living"]);
    }

    /// Once sent, the only honest thing to analyse is what actually went out.
    #[tokio::test]
    async fn a_sent_application_falls_back_to_its_snapshot() {
        let cvs = Arc::new(FakeCvs {
            frozen: Some(a_cv("Frozen")),
            ..Default::default()
        });

        AnalyseApplicationService::new(
            FakeApplications(Some(an_application(Some(Uuid::new_v4())))),
            FakeJobs(Some(a_job())),
            Arc::clone(&cvs),
            None,
        )
        .execute(owner(), Uuid::new_v4(), AnalyseApplicationInput::default())
        .await
        .unwrap();

        assert_eq!(cvs.read.lock().unwrap().as_slice(), &["frozen"]);
    }

    #[tokio::test]
    async fn a_draft_with_no_cv_named_has_nothing_to_analyse() {
        let err = AnalyseApplicationService::new(
            FakeApplications(Some(an_application(None))),
            FakeJobs(Some(a_job())),
            Arc::new(FakeCvs::default()),
            None,
        )
        .execute(owner(), Uuid::new_v4(), AnalyseApplicationInput::default())
        .await
        .unwrap_err();

        assert!(matches!(err, AnalysisError::NoCvToAnalyse));
    }

    #[tokio::test]
    async fn another_users_application_is_not_found() {
        let err = AnalyseApplicationService::new(
            FakeApplications(None),
            FakeJobs(Some(a_job())),
            Arc::new(FakeCvs::default()),
            None,
        )
        .execute(owner(), Uuid::new_v4(), AnalyseApplicationInput::default())
        .await
        .unwrap_err();

        assert!(matches!(err, AnalysisError::ApplicationNotFound));
    }

    // ------------------------------------------------------------------
    // The estimated half
    // ------------------------------------------------------------------

    use crate::career::domain::relevance::{RequirementMatch, Verdict};

    fn req(verdict: Verdict) -> RequirementMatch {
        RequirementMatch {
            text: "Kafka in production".into(),
            verdict,
            evidence: Some("Built the order-events pipeline".into()),
        }
    }

    async fn analyse_with(estimator: Option<Arc<dyn RelevanceEstimator>>) -> MatchAnalysis {
        let cvs = Arc::new(FakeCvs {
            living: Some(a_cv("Backend Engineer")),
            ..Default::default()
        });

        AnalyseApplicationService::new(
            FakeApplications(Some(an_application(None))),
            FakeJobs(Some(a_job())),
            cvs,
            estimator,
        )
        .execute(
            owner(),
            Uuid::new_v4(),
            AnalyseApplicationInput {
                cv_id: Some(Uuid::new_v4()),
            },
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn both_halves_are_reported_when_the_model_answers() {
        let analysis = analyse_with(Some(Arc::new(FakeEstimator(Ok(vec![
            req(Verdict::Met),
            req(Verdict::Missing),
        ])))))
        .await;

        let relevance = analysis
            .relevance
            .expect("the estimated half should be present");
        assert_eq!(relevance.score, 50);
        assert_eq!(relevance.requirements.len(), 2);
        assert!(analysis.relevance_unavailable.is_none());
        assert!(
            analysis.readability.score > 0,
            "the measured half stands alongside it"
        );
    }

    /// The score is computed here from the verdicts, never taken from the
    /// model — so it cannot disagree with the rows a reader is shown.
    #[tokio::test]
    async fn the_score_follows_from_the_verdicts_the_model_gave() {
        let analysis = analyse_with(Some(Arc::new(FakeEstimator(Ok(vec![
            req(Verdict::Met),
            req(Verdict::Met),
            req(Verdict::Partial),
            req(Verdict::Missing),
        ])))))
        .await;

        // 1 + 1 + 0.5 + 0 = 2.5 of 4
        assert_eq!(analysis.relevance.unwrap().score, 63);
    }

    /// The half that was computed correctly must not be lost because the other
    /// one failed. Each failure carries a code so the UI can say something
    /// better than "unavailable".
    #[tokio::test]
    async fn a_failed_estimate_degrades_with_a_reason_rather_than_erroring() {
        for (error, expected) in [
            (RelevanceEstimatorError::QuotaExceeded, "AI_QUOTA_EXCEEDED"),
            (RelevanceEstimatorError::Refused, "AI_REFUSED"),
            (RelevanceEstimatorError::Disabled, "AI_DISABLED"),
            (
                RelevanceEstimatorError::Failed("unreachable".into()),
                "AI_UPSTREAM_ERROR",
            ),
        ] {
            let analysis = analyse_with(Some(Arc::new(FakeEstimator(Err(error))))).await;

            assert!(analysis.relevance.is_none());
            assert_eq!(analysis.relevance_unavailable.as_deref(), Some(expected));
            assert!(
                analysis.readability.score > 0,
                "the measured half must survive a failed estimate"
            );
        }
    }

    /// A deployment with no provider still analyses what it can.
    #[tokio::test]
    async fn no_configured_provider_still_returns_the_measured_half() {
        let analysis = analyse_with(None).await;

        assert!(analysis.relevance.is_none());
        assert_eq!(
            analysis.relevance_unavailable.as_deref(),
            Some("AI_DISABLED")
        );
        assert!(!analysis.readability.checks.is_empty());
    }

    /// An application pointing at a posting that is gone loses the estimate,
    /// not the whole analysis.
    #[tokio::test]
    async fn a_missing_job_costs_the_estimate_not_the_analysis() {
        let cvs = Arc::new(FakeCvs {
            living: Some(a_cv("Backend Engineer")),
            ..Default::default()
        });

        let analysis = AnalyseApplicationService::new(
            FakeApplications(Some(an_application(None))),
            FakeJobs(None),
            cvs,
            Some(
                Arc::new(FakeEstimator(Ok(vec![req(Verdict::Met)]))) as Arc<dyn RelevanceEstimator>
            ),
        )
        .execute(
            owner(),
            Uuid::new_v4(),
            AnalyseApplicationInput {
                cv_id: Some(Uuid::new_v4()),
            },
        )
        .await
        .unwrap();

        assert!(analysis.relevance.is_none());
        assert_eq!(
            analysis.relevance_unavailable.as_deref(),
            Some("JOB_NOT_FOUND")
        );
        assert!(analysis.readability.score > 0);
    }
}
