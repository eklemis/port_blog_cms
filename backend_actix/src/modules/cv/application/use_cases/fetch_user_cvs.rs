use crate::cv::application::ports::outgoing::{CVRepository, CVRepositoryError};
use crate::cv::domain::entities::CVInfo;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum FetchCVError {
    NoCVs, // user exists, but has zero CVs
    RepositoryError(String),
}

/// The FetchCVUseCase orchestrates the domain logic for retrieving a user's CV.
/// TDD approach: we will write tests verifying `execute` behavior using a mock repository.
#[derive(Debug, Clone)]
pub struct FetchCVUseCase<R>
where
    R: CVRepository,
{
    cv_repository: R,
}

impl<R> FetchCVUseCase<R>
where
    R: CVRepository,
{
    /// Construct the use case with a concrete repository implementation.
    pub fn new(repository: R) -> Self {
        Self {
            cv_repository: repository,
        }
    }
}

#[async_trait::async_trait]
pub trait IFetchCVUseCase: Send + Sync {
    async fn execute(&self, user_id: Uuid) -> Result<Vec<CVInfo>, FetchCVError>;
}

#[async_trait::async_trait]
impl<R> IFetchCVUseCase for FetchCVUseCase<R>
where
    R: CVRepository + Sync + Send,
{
    async fn execute(&self, user_id: Uuid) -> Result<Vec<CVInfo>, FetchCVError> {
        let cvs = self
            .cv_repository
            .fetch_cv_by_user_id(user_id)
            .await
            .map_err(|e| match e {
                CVRepositoryError::DatabaseError(msg) => FetchCVError::RepositoryError(msg),
                _ => FetchCVError::RepositoryError("Unknown repository error".to_string()),
            })?;

        if cvs.is_empty() {
            return Err(FetchCVError::NoCVs);
        }

        Ok(cvs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cv::application::ports::outgoing::{
        CVRepository, CVRepositoryError, CreateCVData, UpdateCVData,
    };
    use crate::cv::application::use_cases::fetch_user_cvs::{FetchCVError, FetchCVUseCase};
    use crate::cv::domain::entities::CVInfo;
    use async_trait::async_trait;
    use tokio;
    use uuid;

    // A simple mock repository struct
    #[derive(Default)]
    struct MockCVRepository {
        pub cv_infos: Vec<CVInfo>,
        pub should_fail_db: bool,
    }

    #[async_trait]
    impl CVRepository for MockCVRepository {
        async fn fetch_cv_by_user_id(
            &self,
            _user_id: Uuid,
        ) -> Result<Vec<CVInfo>, CVRepositoryError> {
            if self.should_fail_db {
                return Err(CVRepositoryError::DatabaseError(
                    "DB connection failed".to_string(),
                ));
            }

            Ok(self.cv_infos.clone())
        }
        async fn fetch_cv_by_id(&self, _cv_id: Uuid) -> Result<Option<CVInfo>, CVRepositoryError> {
            unimplemented!()
        }

        async fn create_cv(
            &self,
            _user_id: Uuid,
            _cv_data: CreateCVData,
        ) -> Result<CVInfo, CVRepositoryError> {
            unimplemented!()
            // or return something like:
            // Err(CVRepositoryError::DatabaseError("Not Implemented".into()))
        }
        async fn update_cv(
            &self,
            _user_id: Uuid,
            _cv_data: UpdateCVData,
        ) -> Result<CVInfo, CVRepositoryError> {
            // Temporary stub so that tests compile
            unimplemented!()
        }
    }

    // Test: successful fetch
    #[tokio::test]
    async fn test_fetch_cv_success() {
        // Arrange
        let user_id = Uuid::new_v4();

        let mock_repo = MockCVRepository {
            cv_infos: vec![CVInfo {
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
            }],
            should_fail_db: false,
        };

        let use_case = FetchCVUseCase::new(mock_repo);

        // Act
        let result = use_case.execute(user_id).await;

        // Assert
        assert!(result.is_ok());

        let cv_infos = result.unwrap();
        assert_eq!(cv_infos.len(), 1, "Expected exactly one CV");
        assert_eq!(cv_infos[0].bio, "Test CV");
    }

    // Test: Database error
    #[tokio::test]
    async fn test_fetch_cv_db_error() {
        // Arrange: The mock simulates a DB failure
        let mock_repo = MockCVRepository {
            cv_infos: vec![],
            should_fail_db: true,
        };
        let use_case = FetchCVUseCase::new(mock_repo);
        // Act
        let user_id = uuid::Uuid::new_v4();
        let result = use_case.execute(user_id).await;

        // Assert
        match result {
            Err(FetchCVError::RepositoryError(msg)) => {
                assert_eq!(msg, "DB connection failed");
            }
            _ => panic!("Expected RepositoryError"),
        }
    }

    #[tokio::test]
    async fn test_fetch_cv_no_cvs() {
        let user_id = Uuid::new_v4();

        let mock_repo = MockCVRepository {
            cv_infos: vec![],
            should_fail_db: false,
        };

        let use_case = FetchCVUseCase::new(mock_repo);

        let result = use_case.execute(user_id).await;

        match result {
            Err(FetchCVError::NoCVs) => {}
            _ => panic!("Expected NoCVs error"),
        }
    }
}
