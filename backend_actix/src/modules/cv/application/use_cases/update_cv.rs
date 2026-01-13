use crate::cv::application::ports::outgoing::{CVRepository, CVRepositoryError, UpdateCVData};
use crate::cv::domain::entities::CVInfo;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub enum UpdateCVError {
    CVNotFound,
    RepositoryError(String),
}

#[async_trait]
pub trait IUpdateCVUseCase: Send + Sync {
    async fn execute(
        &self,
        user_id: String,
        cv_data: UpdateCVData,
    ) -> Result<CVInfo, UpdateCVError>;
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
    async fn execute(
        &self,
        user_id: String,
        cv_data: UpdateCVData,
    ) -> Result<CVInfo, UpdateCVError> {
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
    use crate::cv::application::ports::outgoing::{CVRepository, CVRepositoryError, CreateCVData};
    use crate::cv::domain::entities::CVInfo;
    use async_trait::async_trait;
    use tokio;
    use uuid::Uuid;

    // Define a simple mock repository for update tests.
    #[derive(Default)]
    struct MockCVRepository {
        pub existing_cvs: Vec<CVInfo>,
        pub should_fail_update: bool,
        should_fail_create: bool,
    }

    #[async_trait]
    impl CVRepository for MockCVRepository {
        async fn fetch_cv_by_user_id(
            &self,
            _user_id: String,
        ) -> Result<Vec<CVInfo>, CVRepositoryError> {
            if self.existing_cvs.is_empty() {
                return Err(CVRepositoryError::NotFound);
            }
            Ok(self.existing_cvs.clone())
        }

        async fn create_cv(
            &self,
            _user_id: String,
            cv_data: CreateCVData,
        ) -> Result<CVInfo, CVRepositoryError> {
            if self.should_fail_create {
                return Err(CVRepositoryError::DatabaseError(
                    "Create failed".to_string(),
                ));
            }

            // Convert CreateCVData to CVInfo by adding a generated ID
            Ok(CVInfo {
                id: Uuid::new_v4().to_string(), // Generate new ID
                role: cv_data.role,
                bio: cv_data.bio,
                photo_url: cv_data.photo_url,
                core_skills: cv_data.core_skills,
                educations: cv_data.educations,
                experiences: cv_data.experiences,
                highlighted_projects: cv_data.highlighted_projects,
            })
        }

        async fn update_cv(
            &self,
            cv_id: String, // Changed parameter name to cv_id for clarity
            cv_data: UpdateCVData,
        ) -> Result<CVInfo, CVRepositoryError> {
            if self.should_fail_update {
                return Err(CVRepositoryError::DatabaseError(
                    "Update failed".to_string(),
                ));
            }

            // Find the existing CV by ID
            let existing_cv = self
                .existing_cvs
                .iter()
                .find(|cv| cv.id == cv_id)
                .ok_or(CVRepositoryError::NotFound)?;

            // Convert UpdateCVData to CVInfo, keeping the same ID
            Ok(CVInfo {
                id: existing_cv.id.clone(), // Keep the same ID
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
    // Helper for tests
    impl MockCVRepository {
        fn new() -> Self {
            Self {
                existing_cvs: vec![],
                should_fail_update: false,
                should_fail_create: false,
            }
        }

        fn with_existing_cvs(mut self, cvs: Vec<CVInfo>) -> Self {
            self.existing_cvs = cvs;
            self
        }

        fn with_update_failure(mut self) -> Self {
            self.should_fail_update = true;
            self
        }

        fn with_create_failure(mut self) -> Self {
            self.should_fail_create = true;
            self
        }
    }
    #[tokio::test]
    async fn test_update_cv_success() {
        // Arrange: an existing CV is present with a known ID
        let cv_id = Uuid::new_v4().to_string();
        let existing_cv = CVInfo {
            id: cv_id.clone(),
            role: "Software Engineer".to_string(),
            bio: "Old bio".to_string(),
            photo_url: "https://example.com/old.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        let mock_repo = MockCVRepository {
            existing_cvs: vec![existing_cv.clone()],
            should_fail_update: false,
            should_fail_create: false,
        };
        let use_case = UpdateCVUseCase::new(mock_repo);

        // Create UpdateCVData (no id field)
        let update_data = UpdateCVData {
            role: "Senior Software Engineer".to_string(), // Can also update role
            bio: "Updated bio".to_string(),
            photo_url: "https://example.com/new.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        // Act - pass the CV ID (not user ID) and UpdateCVData
        let result = use_case.execute(cv_id.clone(), update_data).await;

        // Assert
        assert!(result.is_ok());
        let updated_cv = result.unwrap();
        assert_eq!(updated_cv.id, cv_id); // ID should remain the same
        assert_eq!(updated_cv.bio, "Updated bio");
        assert_eq!(updated_cv.role, "Senior Software Engineer");
        assert_eq!(updated_cv.photo_url, "https://example.com/new.jpg");
    }

    #[tokio::test]
    async fn test_update_cv_not_found() {
        // Arrange: no existing CV in the repository
        let mock_repo = MockCVRepository {
            existing_cvs: Vec::new(),
            should_fail_update: false,
            should_fail_create: false,
        };
        let use_case = UpdateCVUseCase::new(mock_repo);

        // Use a CV ID that doesn't exist
        let non_existent_cv_id = Uuid::new_v4().to_string();

        // Create UpdateCVData (no id field)
        let update_data = UpdateCVData {
            role: "Software Engineer".to_string(),
            bio: "Updated bio".to_string(),
            photo_url: "https://example.com/new.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        // Act
        let result = use_case.execute(non_existent_cv_id, update_data).await;

        // Assert
        match result {
            Err(UpdateCVError::CVNotFound) => (),
            _ => panic!("Expected CVNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_update_cv_db_error() {
        // Arrange: an existing CV is present with a known ID, but update is forced to fail
        let cv_id = Uuid::new_v4().to_string();
        let existing_cv = CVInfo {
            id: cv_id.clone(),
            role: "Software Engineer".to_string(),
            bio: "Old bio".to_string(),
            photo_url: "https://example.com/old.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        let mock_repo = MockCVRepository {
            existing_cvs: vec![existing_cv],
            should_fail_update: true,
            should_fail_create: false,
        };
        let use_case = UpdateCVUseCase::new(mock_repo);

        // Create UpdateCVData (no id field)
        let update_data = UpdateCVData {
            role: "Senior Software Engineer".to_string(),
            bio: "Updated bio".to_string(),
            photo_url: "https://example.com/new.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        // Act
        let result = use_case.execute(cv_id, update_data).await;

        // Assert
        match result {
            Err(UpdateCVError::RepositoryError(msg)) => {
                assert_eq!(msg, "Update failed");
            }
            _ => panic!("Expected RepositoryError"),
        }
    }
}
