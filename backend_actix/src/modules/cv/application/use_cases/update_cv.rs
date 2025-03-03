use crate::cv::application::ports::outgoing::{CVRepository, CVRepositoryError};
use crate::cv::domain::entities::CVInfo;
use async_trait::async_trait;
use uuid::Uuid;

#[derive(Debug)]
pub enum UpdateCVError {
    CVNotFound,
    RepositoryError(String),
}

#[async_trait]
pub trait IUpdateCVUseCase: Send + Sync {
    async fn execute(&self, user_id: Uuid, cv_data: CVInfo) -> Result<CVInfo, UpdateCVError>;
}

#[derive(Debug, Clone)]
pub struct UpdateCVUseCase<R: CVRepository> {
    repository: R,
}

impl<R: CVRepository> UpdateCVUseCase<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl<R: CVRepository + Sync + Send> IUpdateCVUseCase for UpdateCVUseCase<R> {
    async fn execute(&self, user_id: Uuid, cv_data: CVInfo) -> Result<CVInfo, UpdateCVError> {
        self.repository
            .update_cv(user_id, cv_data)
            .await
            .map_err(|err| match err {
                CVRepositoryError::NotFound => UpdateCVError::CVNotFound,
                CVRepositoryError::DatabaseError(msg) => UpdateCVError::RepositoryError(msg),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cv::application::ports::outgoing::{CVRepository, CVRepositoryError};
    use crate::cv::domain::entities::CVInfo;
    use async_trait::async_trait;
    use tokio;
    use uuid::Uuid;

    // Define a simple mock repository for update tests.
    #[derive(Default)]
    struct MockCVRepository {
        pub existing_cv: Option<CVInfo>,
        pub should_fail_update: bool,
    }

    #[async_trait]
    impl CVRepository for MockCVRepository {
        async fn fetch_cv_by_user_id(&self, _user_id: Uuid) -> Result<CVInfo, CVRepositoryError> {
            if let Some(ref cv) = self.existing_cv {
                Ok(cv.clone())
            } else {
                Err(CVRepositoryError::NotFound)
            }
        }

        async fn create_cv(
            &self,
            _user_id: Uuid,
            cv_data: CVInfo,
        ) -> Result<CVInfo, CVRepositoryError> {
            Ok(cv_data) // Stub for create; not used in update tests.
        }

        async fn update_cv(
            &self,
            _user_id: Uuid,
            cv_data: CVInfo,
        ) -> Result<CVInfo, CVRepositoryError> {
            if self.should_fail_update {
                Err(CVRepositoryError::DatabaseError(
                    "Update failed".to_string(),
                ))
            } else if self.existing_cv.is_none() {
                Err(CVRepositoryError::NotFound)
            } else {
                // Simulate a successful update by returning the new cv_data.
                Ok(cv_data)
            }
        }
    }

    #[tokio::test]
    async fn test_update_cv_success() {
        // Arrange: an existing CV is present.
        let existing_cv = CVInfo {
            bio: "Old bio".to_string(),
            photo_url: "https://example.com/old.jpg".to_string(),
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };
        let mock_repo = MockCVRepository {
            existing_cv: Some(existing_cv),
            should_fail_update: false,
        };
        let use_case = UpdateCVUseCase::new(mock_repo);
        let user_id = Uuid::new_v4();

        let new_cv = CVInfo {
            bio: "Updated bio".to_string(),
            photo_url: "https://example.com/new.jpg".to_string(),
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        // Act
        let result = use_case.execute(user_id, new_cv.clone()).await;

        // Assert
        assert!(result.is_ok());
        let updated_cv = result.unwrap();
        assert_eq!(updated_cv.bio, "Updated bio");
        assert_eq!(updated_cv.photo_url, "https://example.com/new.jpg");
    }

    #[tokio::test]
    async fn test_update_cv_not_found() {
        // Arrange: no existing CV in the repository.
        let mock_repo = MockCVRepository {
            existing_cv: None,
            should_fail_update: false,
        };
        let use_case = UpdateCVUseCase::new(mock_repo);
        let user_id = Uuid::new_v4();

        let new_cv = CVInfo {
            bio: "Updated bio".to_string(),
            photo_url: "https://example.com/new.jpg".to_string(),
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        // Act
        let result = use_case.execute(user_id, new_cv).await;

        // Assert
        match result {
            Err(UpdateCVError::CVNotFound) => (),
            _ => panic!("Expected CVNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_update_cv_db_error() {
        // Arrange: an existing CV is present, but update is forced to fail.
        let existing_cv = CVInfo {
            bio: "Old bio".to_string(),
            photo_url: "https://example.com/old.jpg".to_string(),
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };
        let mock_repo = MockCVRepository {
            existing_cv: Some(existing_cv),
            should_fail_update: true,
        };
        let use_case = UpdateCVUseCase::new(mock_repo);
        let user_id = Uuid::new_v4();

        let new_cv = CVInfo {
            bio: "Updated bio".to_string(),
            photo_url: "https://example.com/new.jpg".to_string(),
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        // Act
        let result = use_case.execute(user_id, new_cv).await;

        // Assert
        match result {
            Err(UpdateCVError::RepositoryError(msg)) => {
                assert_eq!(msg, "Update failed");
            }
            _ => panic!("Expected RepositoryError"),
        }
    }
}
