use crate::auth::application::ports::outgoing::UserQuery;
use crate::cv::application::ports::outgoing::{CVRepository, CVRepositoryError};
use crate::cv::domain::entities::CVInfo;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum FetchCVError {
    UserNotFound, // valid UUID, user does not exist
    NoCVs,        // user exists, but has zero CVs
    RepositoryError(String),
}

/// The FetchCVUseCase orchestrates the domain logic for retrieving a user's CV.
/// TDD approach: we will write tests verifying `execute` behavior using a mock repository.
#[derive(Debug, Clone)]
pub struct FetchCVUseCase<R, U>
where
    R: CVRepository,
    U: UserQuery,
{
    cv_repository: R,
    user_query: U,
}

impl<R, U> FetchCVUseCase<R, U>
where
    R: CVRepository,
    U: UserQuery,
{
    /// Construct the use case with a concrete repository implementation.
    pub fn new(repository: R, query: U) -> Self {
        Self {
            cv_repository: repository,
            user_query: query,
        }
    }
}

#[async_trait::async_trait]
pub trait IFetchCVUseCase: Send + Sync {
    async fn execute(&self, user_id: Uuid) -> Result<Vec<CVInfo>, FetchCVError>;
}

#[async_trait::async_trait]
impl<R, U> IFetchCVUseCase for FetchCVUseCase<R, U>
where
    R: CVRepository + Sync + Send,
    U: UserQuery + Sync + Send,
{
    async fn execute(&self, user_id: Uuid) -> Result<Vec<CVInfo>, FetchCVError> {
        let user = self
            .user_query
            .find_by_id(user_id)
            .await
            .map_err(FetchCVError::RepositoryError)?;

        if user.is_none() {
            return Err(FetchCVError::UserNotFound);
        }

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
    use crate::auth::application::domain::entities::User;
    use crate::cv::application::ports::outgoing::{
        CVRepository, CVRepositoryError, CreateCVData, UpdateCVData,
    };
    use crate::cv::application::use_cases::fetch_cv::{FetchCVError, FetchCVUseCase};
    use crate::cv::domain::entities::CVInfo;
    use async_trait::async_trait;
    use chrono::Utc;
    use tokio;
    use uuid;

    // A simple mock repository struct
    #[derive(Default)]
    struct MockCVRepository {
        pub cv_infos: Vec<CVInfo>,
        pub should_fail_db: bool,
    }

    #[derive(Default)]
    struct MockUserQuery {
        user_exists: bool,
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

    #[async_trait]
    impl UserQuery for MockUserQuery {
        async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>, String> {
            if self.user_exists {
                let now = Utc::now();
                Ok(Some(User {
                    id: user_id,
                    username: "testuser".to_string(),
                    email: "test@example.com".to_string(),
                    password_hash: "hashed_password".to_string(),
                    created_at: now,
                    updated_at: now,
                    is_verified: false,
                    is_deleted: false,
                }))
            } else {
                Ok(None)
            }
        }
        async fn find_by_email(&self, email: &str) -> Result<Option<User>, String> {
            unimplemented!()
        }
        async fn find_by_username(&self, username: &str) -> Result<Option<User>, String> {
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
                id: Uuid::new_v4(), // CV ID, not user ID
                role: "Software Engineer".to_string(),
                bio: "Mocked CV data...".to_string(),
                photo_url: "https://example.com/old.jpg".to_string(),
                core_skills: vec![],
                educations: vec![],
                experiences: vec![],
                highlighted_projects: vec![],
            }],
            should_fail_db: false,
        };

        let mock_user_query = MockUserQuery { user_exists: true };

        let use_case = FetchCVUseCase::new(mock_repo, mock_user_query);

        // Act
        let result = use_case.execute(user_id).await;

        // Assert
        assert!(result.is_ok());

        let cv_infos = result.unwrap();
        assert_eq!(cv_infos.len(), 1, "Expected exactly one CV");
        assert_eq!(cv_infos[0].bio, "Mocked CV data...");
    }

    // Test: Database error
    #[tokio::test]
    async fn test_fetch_cv_db_error() {
        // Arrange: The mock simulates a DB failure
        let mock_repo = MockCVRepository {
            cv_infos: vec![],
            should_fail_db: true,
        };
        let mock_user_query = MockUserQuery { user_exists: true };
        let use_case = FetchCVUseCase::new(mock_repo, mock_user_query);
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

        let mock_user_query = MockUserQuery { user_exists: true };

        let use_case = FetchCVUseCase::new(mock_repo, mock_user_query);

        let result = use_case.execute(user_id).await;

        match result {
            Err(FetchCVError::NoCVs) => {}
            _ => panic!("Expected NoCVs error"),
        }
    }
    #[tokio::test]
    async fn test_fetch_cv_user_not_found() {
        let user_id = Uuid::new_v4();

        let mock_repo = MockCVRepository {
            cv_infos: vec![],
            should_fail_db: false,
        };

        let mock_user_query = MockUserQuery { user_exists: false };

        let use_case = FetchCVUseCase::new(mock_repo, mock_user_query);

        let result = use_case.execute(user_id).await;

        match result {
            Err(FetchCVError::UserNotFound) => {}
            _ => panic!("Expected UserNotFound error"),
        }
    }
}
