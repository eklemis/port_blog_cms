use crate::cv::application::ports::outgoing::{CVRepository, CVRepositoryError};
use crate::cv::domain::entities::CVInfo;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum FetchCVByIdError {
    CVNotFound,
    RepositoryError(String),
}

#[derive(Debug, Clone)]
pub struct FetchCVByIdUseCase<R>
where
    R: CVRepository,
{
    cv_repository: R,
}
impl<R> FetchCVByIdUseCase<R>
where
    R: CVRepository,
{
    pub fn new(repository: R) -> Self {
        Self {
            cv_repository: repository,
        }
    }
}

#[async_trait::async_trait]
pub trait IFetchCVByIdUseCase: Send + Sync {
    async fn execute(&self, user_id: Uuid, cv_id: Uuid) -> Result<CVInfo, FetchCVByIdError>;
}

#[async_trait::async_trait]
impl<R> IFetchCVByIdUseCase for FetchCVByIdUseCase<R>
where
    R: CVRepository + Send + Sync,
{
    async fn execute(&self, user_id: Uuid, cv_id: Uuid) -> Result<CVInfo, FetchCVByIdError> {
        // 2️⃣ Fetch CV by ID
        let cv = self
            .cv_repository
            .fetch_cv_by_id(cv_id)
            .await
            .map_err(|e| match e {
                CVRepositoryError::DatabaseError(msg) => FetchCVByIdError::RepositoryError(msg),
                _ => FetchCVByIdError::RepositoryError("Unknown repository error".to_string()),
            })?;

        let cv = match cv {
            Some(cv) => cv,
            None => return Err(FetchCVByIdError::CVNotFound),
        };

        // 3️⃣ Enforce ownership
        if cv.user_id != user_id {
            // IMPORTANT:
            // Do NOT leak existence of CVs belonging to other users
            return Err(FetchCVByIdError::CVNotFound);
        }

        Ok(cv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[derive(Clone)]
    struct MockCVRepository {
        cv: Option<CVInfo>,
        should_fail: bool,
    }
    #[async_trait::async_trait]
    impl CVRepository for MockCVRepository {
        async fn fetch_cv_by_user_id(
            &self,
            _user_id: Uuid,
        ) -> Result<Vec<CVInfo>, CVRepositoryError> {
            unimplemented!("Not used in FetchCVByIdUseCase tests")
        }

        async fn fetch_cv_by_id(&self, _cv_id: Uuid) -> Result<Option<CVInfo>, CVRepositoryError> {
            if self.should_fail {
                return Err(CVRepositoryError::DatabaseError(
                    "cv repo failed".to_string(),
                ));
            }

            Ok(self.cv.clone())
        }

        async fn create_cv(
            &self,
            _user_id: Uuid,
            _cv_data: crate::cv::application::ports::outgoing::CreateCVData,
        ) -> Result<CVInfo, CVRepositoryError> {
            unimplemented!()
        }

        async fn update_cv(
            &self,
            _user_id: Uuid,
            _cv_data: crate::cv::application::ports::outgoing::UpdateCVData,
        ) -> Result<CVInfo, CVRepositoryError> {
            unimplemented!()
        }
    }

    fn sample_cv(user_id: Uuid) -> CVInfo {
        CVInfo {
            id: Uuid::new_v4(),
            user_id,
            display_name: "Gandalf Wood".to_string(),
            role: "Engineer".to_string(),
            bio: "Test CV".to_string(),
            photo_url: "".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        }
    }

    #[tokio::test]
    async fn cv_not_found() {
        let user_id = Uuid::new_v4();

        let use_case = FetchCVByIdUseCase::new(MockCVRepository {
            cv: None,
            should_fail: false,
        });

        let result = use_case.execute(user_id, Uuid::new_v4()).await;

        assert!(matches!(result, Err(FetchCVByIdError::CVNotFound)));
    }

    #[tokio::test]
    async fn cv_belongs_to_other_user() {
        let user_id = Uuid::new_v4();
        let other_user_id = Uuid::new_v4();

        let use_case = FetchCVByIdUseCase::new(MockCVRepository {
            cv: Some(sample_cv(other_user_id)),
            should_fail: false,
        });

        let result = use_case.execute(user_id, Uuid::new_v4()).await;

        assert!(matches!(result, Err(FetchCVByIdError::CVNotFound)));
    }

    #[tokio::test]
    async fn success_when_cv_belongs_to_user() {
        let user_id = Uuid::new_v4();
        let cv = sample_cv(user_id);

        let use_case = FetchCVByIdUseCase::new(MockCVRepository {
            cv: Some(cv.clone()),
            should_fail: false,
        });

        let result = use_case.execute(user_id, cv.id).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, cv.id);
    }

    #[tokio::test]
    async fn repository_failure() {
        let user_id = Uuid::new_v4();

        let use_case = FetchCVByIdUseCase::new(MockCVRepository {
            cv: None,
            should_fail: true,
        });

        let result = use_case.execute(user_id, Uuid::new_v4()).await;

        assert!(matches!(result, Err(FetchCVByIdError::RepositoryError(_))));
    }
}
