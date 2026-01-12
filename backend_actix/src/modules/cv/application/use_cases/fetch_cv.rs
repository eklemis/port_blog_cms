use crate::cv::application::ports::outgoing::{CVRepository, CVRepositoryError};
use crate::cv::domain::entities::CVInfo;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum FetchCVError {
    CVNotFound,
    RepositoryError(String),
    // More variants if needed
}

/// The FetchCVUseCase orchestrates the domain logic for retrieving a user's CV.
/// TDD approach: we will write tests verifying `execute` behavior using a mock repository.
#[derive(Debug, Clone)]
pub struct FetchCVUseCase<R: CVRepository> {
    repository: R,
}

impl<R: CVRepository> FetchCVUseCase<R> {
    /// Construct the use case with a concrete repository implementation.
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

#[async_trait::async_trait]
pub trait IFetchCVUseCase: Send + Sync {
    async fn execute(&self, user_id: Uuid) -> Result<CVInfo, FetchCVError>;
}

#[async_trait::async_trait]
impl<R: CVRepository + Sync + Send> IFetchCVUseCase for FetchCVUseCase<R> {
    async fn execute(&self, user_id: Uuid) -> Result<CVInfo, FetchCVError> {
        match self.repository.fetch_cv_by_user_id(user_id).await {
            Ok(cv_info) => Ok(cv_info),
            Err(CVRepositoryError::NotFound) => Err(FetchCVError::CVNotFound),
            Err(CVRepositoryError::DatabaseError(e)) => Err(FetchCVError::RepositoryError(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cv::application::ports::outgoing::{CVRepository, CVRepositoryError};
    use crate::cv::application::use_cases::fetch_cv::{FetchCVError, FetchCVUseCase};
    use crate::cv::domain::entities::CVInfo;
    use async_trait::async_trait;
    use tokio;
    use uuid::Uuid;

    // A simple mock repository struct
    #[derive(Default)]
    struct MockCVRepository {
        pub cv_info: Option<CVInfo>,
        pub should_fail_db: bool,
    }

    #[async_trait]
    impl CVRepository for MockCVRepository {
        async fn fetch_cv_by_user_id(&self, _user_id: Uuid) -> Result<CVInfo, CVRepositoryError> {
            if self.should_fail_db {
                Err(CVRepositoryError::DatabaseError(
                    "DB connection failed".to_string(),
                ))
            } else if let Some(ref cv) = self.cv_info {
                Ok(cv.clone())
            } else {
                Err(CVRepositoryError::NotFound)
            }
        }
        async fn create_cv(
            &self,
            _user_id: Uuid,
            _cv_data: CVInfo,
        ) -> Result<CVInfo, CVRepositoryError> {
            unimplemented!()
            // or return something like:
            // Err(CVRepositoryError::DatabaseError("Not Implemented".into()))
        }
        async fn update_cv(
            &self,
            _user_id: Uuid,
            _cv_data: CVInfo,
        ) -> Result<CVInfo, CVRepositoryError> {
            // Temporary stub so that tests compile
            unimplemented!()
        }
    }

    // Test: successful fetch
    #[tokio::test]
    async fn test_fetch_cv_success() {
        // Arrange: Create a mock with a valid CVInfo
        let mock_repo = MockCVRepository {
            cv_info: Some(CVInfo {
                role: "Software Engineer".to_string(),
                bio: "Mocked CV data...".to_string(),
                photo_url: "https://example.com/old.jpg".to_string(),
                core_skills: vec![],
                educations: vec![],
                experiences: vec![],
                highlighted_projects: vec![],
            }),
            should_fail_db: false,
        };
        let use_case = FetchCVUseCase::new(mock_repo);

        // Act: Execute with some dummy user ID
        let user_id = Uuid::new_v4();
        let result = use_case.execute(user_id).await;

        // Assert: We expect a successful result
        assert!(result.is_ok());
        let cv_info = result.unwrap();
        assert_eq!(cv_info.bio, "Mocked CV data...");
    }

    // Test: CV not found
    #[tokio::test]
    async fn test_fetch_cv_not_found() {
        // Arrange: The mock's cv_info is None
        let mock_repo = MockCVRepository {
            cv_info: None,
            should_fail_db: false,
        };
        let use_case = FetchCVUseCase::new(mock_repo);

        // Act
        let user_id = Uuid::new_v4();
        let result = use_case.execute(user_id).await;

        // Assert
        match result {
            Err(FetchCVError::CVNotFound) => (),
            _ => panic!("Expected CVNotFound error"),
        }
    }

    // Test: Database error
    #[tokio::test]
    async fn test_fetch_cv_db_error() {
        // Arrange: The mock simulates a DB failure
        let mock_repo = MockCVRepository {
            cv_info: None,
            should_fail_db: true,
        };
        let use_case = FetchCVUseCase::new(mock_repo);

        // Act
        let user_id = Uuid::new_v4();
        let result = use_case.execute(user_id).await;

        // Assert
        match result {
            Err(FetchCVError::RepositoryError(msg)) => {
                assert_eq!(msg, "DB connection failed");
            }
            _ => panic!("Expected RepositoryError"),
        }
    }
}
