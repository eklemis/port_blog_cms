use crate::cv::application::ports::outgoing::{CVRepository, CVRepositoryError, CreateCVData};
use crate::cv::domain::entities::CVInfo;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub enum CreateCVError {
    AlreadyExists, //Only allow 1 CV per user and it's found
    RepositoryError(String),
}

/// An interface for the create CV use case
#[async_trait]
pub trait ICreateCVUseCase: Send + Sync {
    async fn execute(
        &self,
        user_id: String,
        cv_data: CreateCVData,
    ) -> Result<CVInfo, CreateCVError>;
}

/// Concrete implementation of the create CV use case
#[derive(Debug, Clone)]
pub struct CreateCVUseCase<R: CVRepository> {
    repository: R,
}

impl<R: CVRepository> CreateCVUseCase<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl<R: CVRepository + Sync + Send> ICreateCVUseCase for CreateCVUseCase<R> {
    async fn execute(
        &self,
        user_id: String,
        cv_data: CreateCVData,
    ) -> Result<CVInfo, CreateCVError> {
        // Now, users are allowed to have more than one cv

        // Attempt creation
        match self.repository.create_cv(user_id, cv_data).await {
            Ok(created_cv) => Ok(created_cv),
            Err(CVRepositoryError::DatabaseError(e)) => Err(CreateCVError::RepositoryError(e)),
            Err(_) => Err(CreateCVError::RepositoryError(
                "Unknown repo error".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cv::application::ports::outgoing::{CVRepository, CVRepositoryError, UpdateCVData};
    use crate::cv::domain::entities::CVInfo;
    use async_trait::async_trait;
    use tokio;
    use uuid::Uuid;

    #[derive(Default)]
    struct MockCVRepository {
        pub existing_cvs: Vec<CVInfo>,
        pub should_fail_on_create: bool,
    }

    #[async_trait]
    impl CVRepository for MockCVRepository {
        async fn fetch_cv_by_user_id(
            &self,
            _user_id: String,
        ) -> Result<Vec<CVInfo>, CVRepositoryError> {
            if self.existing_cvs.len() > 0 {
                Ok(self.existing_cvs.clone())
            } else {
                Err(CVRepositoryError::NotFound)
            }
        }

        async fn create_cv(
            &self,
            _user_id: String,
            cv_data: CreateCVData,
        ) -> Result<CVInfo, CVRepositoryError> {
            if self.should_fail_on_create {
                Err(CVRepositoryError::DatabaseError(
                    "DB insert failed".to_string(),
                ))
            } else {
                Ok(CVInfo {
                    id: Uuid::new_v4().to_string(),
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
            _user_id: String,
            _cv_data: UpdateCVData,
        ) -> Result<CVInfo, CVRepositoryError> {
            // Temporary stub so that tests compile
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_create_cv_success() {
        // Arrange
        let mock_repo = MockCVRepository {
            existing_cvs: Vec::new(), // No existing CV
            should_fail_on_create: false,
        };
        let use_case = CreateCVUseCase::new(mock_repo);

        // Act
        let user_id = Uuid::new_v4().to_string();
        let new_cv_data = CreateCVData {
            role: "Software Engineer".to_string(),
            bio: "My new bio".to_string(),
            photo_url: "https://example.com/old.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        let result = use_case.execute(user_id, new_cv_data.clone()).await;

        // Assert
        assert!(result.is_ok());
        let created_cv = result.unwrap();
        assert_eq!(created_cv.bio, "My new bio");
        assert_eq!(created_cv.role, "Software Engineer");
        // other checks...
    }

    #[tokio::test]
    async fn test_create_cv_db_error() {
        // Arrange: Database fails on create
        let mock_repo = MockCVRepository {
            existing_cvs: Vec::new(),
            should_fail_on_create: true,
        };
        let use_case = CreateCVUseCase::new(mock_repo);

        // Act
        let user_id = Uuid::new_v4().to_string();
        let new_cv_data = CreateCVData {
            role: "Software Engineer".to_string(),
            bio: "Old bio".to_string(),
            photo_url: "https://example.com/old.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };
        let result = use_case.execute(user_id, new_cv_data).await;

        // Assert
        match result {
            Err(CreateCVError::RepositoryError(msg)) => {
                assert_eq!(msg, "DB insert failed");
            }
            _ => panic!("Expected RepositoryError"),
        }
    }

    #[tokio::test]
    async fn test_create_cv_unknown_repository_error() {
        // Arrange: Mock repository that returns a non-DatabaseError variant
        #[derive(Default)]
        struct MockCVRepositoryUnknownError;

        #[async_trait]
        impl CVRepository for MockCVRepositoryUnknownError {
            async fn fetch_cv_by_user_id(
                &self,
                _user_id: String,
            ) -> Result<Vec<CVInfo>, CVRepositoryError> {
                Err(CVRepositoryError::NotFound)
            }

            async fn create_cv(
                &self,
                _user_id: String,
                _cv_data: CreateCVData,
            ) -> Result<CVInfo, CVRepositoryError> {
                // Return NotFound instead of DatabaseError to trigger catch-all
                Err(CVRepositoryError::NotFound)
            }

            async fn update_cv(
                &self,
                _user_id: String,
                _cv_data: UpdateCVData,
            ) -> Result<CVInfo, CVRepositoryError> {
                unimplemented!()
            }
        }

        let mock_repo = MockCVRepositoryUnknownError;
        let use_case = CreateCVUseCase::new(mock_repo);

        // Act
        let user_id = Uuid::new_v4().to_string();
        let new_cv_data = CreateCVData {
            role: "Software Engineer".to_string(),
            bio: "Old bio".to_string(),
            photo_url: "https://example.com/old.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };
        let result = use_case.execute(user_id, new_cv_data).await;

        // Assert - Should return RepositoryError with "Unknown repo error"
        match result {
            Err(CreateCVError::RepositoryError(msg)) => {
                assert_eq!(msg, "Unknown repo error");
            }
            _ => panic!(
                "Expected RepositoryError with 'Unknown repo error', got: {:?}",
                result
            ),
        }
    }
}
