//! Producing a match analysis.
//!
//! Only the measured half exists today. The estimated half needs the AI proxy;
//! this reports `relevance: null` until then rather than inventing a number.

use async_trait::async_trait;
use chrono::{Datelike, Utc};
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::career::application::ports::incoming::use_cases::{
    AnalyseApplicationInput, AnalyseApplicationUseCase, AnalysisError, MatchAnalysis,
};
use crate::career::application::ports::outgoing::{ApplicationStore, CvReader};
use crate::career::domain::readability;
use crate::cv::domain::entities::CVInfo;

/// Implements the corresponding use-case contract.
pub struct AnalyseApplicationService<A, C> {
    applications: A,
    cvs: C,
}

impl<A, C> AnalyseApplicationService<A, C> {
    /// Builds it from the ports it depends on.
    pub fn new(applications: A, cvs: C) -> Self {
        Self { applications, cvs }
    }
}

#[async_trait]
impl<A, C> AnalyseApplicationUseCase for AnalyseApplicationService<A, C>
where
    A: ApplicationStore + Send + Sync,
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

        Ok(MatchAnalysis {
            readability: readability::assess(&cv, Utc::now().year()),
            // Deliberately null rather than absent-with-a-default. See the
            // field's documentation: null means "not computed".
            relevance: None,
        })
    }
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

    /// The half that exists is reported; the half that does not is `null`,
    /// not zero. A client rendering a zero would tell someone their CV
    /// matched nothing.
    #[tokio::test]
    async fn relevance_is_null_rather_than_zero_until_the_model_half_exists() {
        let cvs = Arc::new(FakeCvs {
            living: Some(a_cv("Backend Engineer")),
            ..Default::default()
        });

        let analysis =
            AnalyseApplicationService::new(FakeApplications(Some(an_application(None))), cvs)
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
            Arc::clone(&cvs),
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
            Arc::clone(&cvs),
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
            Arc::new(FakeCvs::default()),
        )
        .execute(owner(), Uuid::new_v4(), AnalyseApplicationInput::default())
        .await
        .unwrap_err();

        assert!(matches!(err, AnalysisError::NoCvToAnalyse));
    }

    #[tokio::test]
    async fn another_users_application_is_not_found() {
        let err =
            AnalyseApplicationService::new(FakeApplications(None), Arc::new(FakeCvs::default()))
                .execute(owner(), Uuid::new_v4(), AnalyseApplicationInput::default())
                .await
                .unwrap_err();

        assert!(matches!(err, AnalysisError::ApplicationNotFound));
    }
}
