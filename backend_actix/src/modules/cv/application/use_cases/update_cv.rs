use crate::cv::application::ports::outgoing::{CVRepository, CVRepositoryError, UpdateCVData};
use crate::cv::domain::entities::CVInfo;
use async_trait::async_trait;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum UpdateCVError {
    CVNotFound,
    RepositoryError(String),
}

#[async_trait]
pub trait IUpdateCVUseCase: Send + Sync {
    async fn execute(
        &self,
        user_id: Uuid,
        cv_id: Uuid,
        data: UpdateCVData,
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

#[async_trait::async_trait]
impl<R> IUpdateCVUseCase for UpdateCVUseCase<R>
where
    R: CVRepository + Send + Sync,
{
    async fn execute(
        &self,
        user_id: Uuid,
        cv_id: Uuid,
        cv_data: UpdateCVData,
    ) -> Result<CVInfo, UpdateCVError> {
        // 1️⃣ Fetch CV by ID
        let cv = self
            .repository
            .fetch_cv_by_id(cv_id)
            .await
            .map_err(|err| match err {
                CVRepositoryError::DatabaseError(msg) => UpdateCVError::RepositoryError(msg),
                _ => UpdateCVError::RepositoryError("Unknown repository error".to_string()),
            })?;

        let cv = match cv {
            Some(cv) => cv,
            None => return Err(UpdateCVError::CVNotFound),
        };

        // 2️⃣ Enforce ownership
        if cv.user_id != user_id {
            // Do NOT leak existence of CVs belonging to other users
            return Err(UpdateCVError::CVNotFound);
        }

        // 3️⃣ Perform update
        self.repository
            .update_cv(cv_id, cv_data)
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
        pub should_fail_create: bool,
    }

    #[async_trait]
    impl CVRepository for MockCVRepository {
        async fn fetch_cv_by_user_id(
            &self,
            user_id: Uuid,
        ) -> Result<Vec<CVInfo>, CVRepositoryError> {
            Ok(self
                .existing_cvs
                .iter()
                .filter(|cv| cv.user_id == user_id)
                .cloned()
                .collect())
        }

        async fn fetch_cv_by_id(&self, cv_id: Uuid) -> Result<Option<CVInfo>, CVRepositoryError> {
            Ok(self.existing_cvs.iter().find(|cv| cv.id == cv_id).cloned())
        }

        async fn update_cv(
            &self,
            cv_id: Uuid,
            cv_data: UpdateCVData,
        ) -> Result<CVInfo, CVRepositoryError> {
            if self.should_fail_update {
                return Err(CVRepositoryError::DatabaseError(
                    "Update failed".to_string(),
                ));
            }

            let existing = self
                .existing_cvs
                .iter()
                .find(|cv| cv.id == cv_id)
                .cloned()
                .ok_or(CVRepositoryError::NotFound)?;

            Ok(CVInfo {
                id: existing.id,
                user_id: existing.user_id,
                role: cv_data.role,
                bio: cv_data.bio,
                photo_url: cv_data.photo_url,
                core_skills: cv_data.core_skills,
                educations: cv_data.educations,
                experiences: cv_data.experiences,
                highlighted_projects: cv_data.highlighted_projects,
            })
        }

        async fn create_cv(
            &self,
            _user_id: Uuid,
            _cv_data: CreateCVData,
        ) -> Result<CVInfo, CVRepositoryError> {
            if self.should_fail_create {
                return Err(CVRepositoryError::DatabaseError(
                    "Create failed".to_string(),
                ));
            }

            unimplemented!("Create is not used in UpdateCV tests")
        }
    }

    #[tokio::test]
    async fn test_update_cv_success() {
        // Arrange: an existing CV is present with a known ID
        let cv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let existing_cv = CVInfo {
            id: cv_id,
            user_id: user_id,
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
        let result = use_case.execute(user_id, cv_id, update_data).await;

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
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        // No existing CVs in repository
        let mock_repo = MockCVRepository {
            existing_cvs: vec![],
            should_fail_update: false,
            should_fail_create: false,
        };

        let use_case = UpdateCVUseCase::new(mock_repo);

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
        let result = use_case.execute(user_id, cv_id, update_data).await;

        // Assert
        match result {
            Err(UpdateCVError::CVNotFound) => (),
            other => panic!("Expected CVNotFound error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_update_cv_db_error() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        // Existing CV belongs to the user
        let existing_cv = CVInfo {
            id: cv_id,
            user_id,
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
            should_fail_update: true, // force DB error
            should_fail_create: false,
        };

        let use_case = UpdateCVUseCase::new(mock_repo);

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
        let result = use_case.execute(user_id, cv_id, update_data).await;

        // Assert
        match result {
            Err(UpdateCVError::RepositoryError(msg)) => {
                assert_eq!(msg, "Update failed");
            }
            other => panic!("Expected RepositoryError, got {:?}", other),
        }
    }
}
