use crate::auth::application::ports::outgoing::UserQuery;
use crate::cv::application::ports::outgoing::{CVRepository, CVRepositoryError, CreateCVData};
use crate::cv::domain::entities::CVInfo;
use async_trait::async_trait;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum CreateCVError {
    UserNotFound,
    RepositoryError(String),
}

/// An interface for the create CV use case
#[async_trait]
pub trait ICreateCVUseCase: Send + Sync {
    async fn execute(&self, user_id: Uuid, cv_data: CreateCVData) -> Result<CVInfo, CreateCVError>;
}

/// Concrete implementation of the create CV use case
pub struct CreateCVUseCase<R, U>
where
    R: CVRepository,
    U: UserQuery,
{
    cv_repository: R,
    user_query: U,
}

impl<R, U> CreateCVUseCase<R, U>
where
    R: CVRepository,
    U: UserQuery,
{
    pub fn new(cv_repository: R, user_query: U) -> Self {
        Self {
            cv_repository,
            user_query,
        }
    }
}

#[async_trait]
impl<R, U> ICreateCVUseCase for CreateCVUseCase<R, U>
where
    R: CVRepository + Sync + Send,
    U: UserQuery + Sync + Send,
{
    async fn execute(&self, user_id: Uuid, cv_data: CreateCVData) -> Result<CVInfo, CreateCVError> {
        let user = self
            .user_query
            .find_by_id(user_id)
            .await
            .map_err(CreateCVError::RepositoryError)?;

        if user.is_none() {
            return Err(CreateCVError::UserNotFound);
        }

        self.cv_repository
            .create_cv(user_id, cv_data)
            .await
            .map_err(|e| match e {
                CVRepositoryError::DatabaseError(msg) => CreateCVError::RepositoryError(msg),
                _ => CreateCVError::RepositoryError("Unknown repo error".to_string()),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cv::application::ports::outgoing::{CVRepository, CVRepositoryError, UpdateCVData};
    use crate::cv::domain::entities::CVInfo;
    use crate::modules::auth::application::domain::entities::User;
    use crate::modules::auth::application::ports::outgoing::UserQuery;
    use async_trait::async_trait;
    use tokio;
    use uuid::Uuid;

    // -----------------------------
    // Mock CV Repository
    // -----------------------------

    #[derive(Default)]
    struct MockCVRepository {
        pub should_fail_on_create: bool,
        pub return_unknown_error: bool,
    }

    #[async_trait]
    impl CVRepository for MockCVRepository {
        async fn fetch_cv_by_user_id(
            &self,
            _user_id: Uuid,
        ) -> Result<Vec<CVInfo>, CVRepositoryError> {
            Ok(vec![])
        }

        async fn create_cv(
            &self,
            _user_id: Uuid,
            cv_data: CreateCVData,
        ) -> Result<CVInfo, CVRepositoryError> {
            if self.return_unknown_error {
                Err(CVRepositoryError::NotFound)
            } else if self.should_fail_on_create {
                Err(CVRepositoryError::DatabaseError(
                    "DB insert failed".to_string(),
                ))
            } else {
                Ok(CVInfo {
                    id: Uuid::new_v4(),
                    role: cv_data.role,
                    bio: cv_data.bio,
                    photo_url: cv_data.photo_url,
                    core_skills: cv_data.core_skills,
                    educations: cv_data.educations,
                    experiences: cv_data.experiences,
                    highlighted_projects: cv_data.highlighted_projects,
                })
            }
        }

        async fn update_cv(
            &self,
            _user_id: Uuid,
            _cv_data: UpdateCVData,
        ) -> Result<CVInfo, CVRepositoryError> {
            unimplemented!()
        }
    }

    // -----------------------------
    // Mock UserQuery
    // -----------------------------

    #[derive(Default)]
    struct MockUserQuery {
        pub user_exists: bool,
    }

    #[async_trait]
    impl UserQuery for MockUserQuery {
        async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>, String> {
            if self.user_exists {
                Ok(Some(User {
                    id: user_id,
                    username: "deleted_user".to_string(),
                    email: "deleted@example.com".to_string(),
                    password_hash: "old_hash".to_string(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    is_verified: false,
                    is_deleted: false,
                }))
            } else {
                Ok(None)
            }
        }

        async fn find_by_email(&self, _email: &str) -> Result<Option<User>, String> {
            Ok(None)
        }

        async fn find_by_username(&self, _username: &str) -> Result<Option<User>, String> {
            Ok(None)
        }
    }

    // -----------------------------
    // Tests
    // -----------------------------

    #[tokio::test]
    async fn test_create_cv_success() {
        let cv_repo = MockCVRepository::default();
        let user_query = MockUserQuery { user_exists: true };

        let use_case = CreateCVUseCase::new(cv_repo, user_query);

        let user_id = Uuid::new_v4();
        let new_cv_data = CreateCVData {
            role: "Software Engineer".to_string(),
            bio: "My new bio".to_string(),
            photo_url: "https://example.com/old.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        let result = use_case.execute(user_id, new_cv_data).await;

        assert!(result.is_ok());
        let created_cv = result.unwrap();
        assert_eq!(created_cv.role, "Software Engineer");
        assert_eq!(created_cv.bio, "My new bio");
    }

    #[tokio::test]
    async fn test_create_cv_db_error() {
        let cv_repo = MockCVRepository {
            should_fail_on_create: true,
            return_unknown_error: false,
        };
        let user_query = MockUserQuery { user_exists: true };

        let use_case = CreateCVUseCase::new(cv_repo, user_query);

        let user_id = Uuid::new_v4();
        let cv_data = CreateCVData {
            role: "Software Engineer".to_string(),
            bio: "Old bio".to_string(),
            photo_url: "https://example.com/old.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        let result = use_case.execute(user_id, cv_data).await;

        match result {
            Err(CreateCVError::RepositoryError(msg)) => {
                assert_eq!(msg, "DB insert failed");
            }
            _ => panic!("Expected RepositoryError"),
        }
    }

    #[tokio::test]
    async fn test_create_cv_unknown_repository_error() {
        let cv_repo = MockCVRepository {
            should_fail_on_create: false,
            return_unknown_error: true,
        };
        let user_query = MockUserQuery { user_exists: true };

        let use_case = CreateCVUseCase::new(cv_repo, user_query);

        let user_id = Uuid::new_v4();
        let cv_data = CreateCVData {
            role: "Software Engineer".to_string(),
            bio: "Old bio".to_string(),
            photo_url: "https://example.com/old.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        let result = use_case.execute(user_id, cv_data).await;

        match result {
            Err(CreateCVError::RepositoryError(msg)) => {
                assert_eq!(msg, "Unknown repo error");
            }
            _ => panic!("Expected RepositoryError with 'Unknown repo error'"),
        }
    }

    // -----------------------------
    // NEW (REQUIRED) TEST
    // -----------------------------

    #[tokio::test]
    async fn test_create_cv_user_not_found() {
        let cv_repo = MockCVRepository::default();
        let user_query = MockUserQuery { user_exists: false };

        let use_case = CreateCVUseCase::new(cv_repo, user_query);

        let user_id = Uuid::new_v4();
        let cv_data = CreateCVData {
            role: "Software Engineer".to_string(),
            bio: "Bio".to_string(),
            photo_url: "url".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        let result = use_case.execute(user_id, cv_data).await;

        match result {
            Err(CreateCVError::UserNotFound) => {}
            _ => panic!("Expected UserNotFound error"),
        }
    }
}
