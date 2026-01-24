use crate::cv::application::ports::outgoing::{
    CVRepository, CVRepositoryError, PatchCVData, UpdateCVData,
};
use crate::cv::domain::entities::CVInfo;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum PatchCVError {
    CVNotFound,
    RepositoryError(String),
}

#[async_trait::async_trait]
pub trait IPatchCVUseCase: Send + Sync {
    async fn execute(
        &self,
        user_id: Uuid,
        cv_id: Uuid,
        data: PatchCVData,
    ) -> Result<CVInfo, PatchCVError>;
}

#[derive(Debug, Clone)]
pub struct PatchCVUseCase<R: CVRepository> {
    repository: R,
}

impl<R: CVRepository> PatchCVUseCase<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

#[async_trait::async_trait]
impl<R> IPatchCVUseCase for PatchCVUseCase<R>
where
    R: CVRepository + Send + Sync,
{
    async fn execute(
        &self,
        user_id: Uuid,
        cv_id: Uuid,
        data: PatchCVData,
    ) -> Result<CVInfo, PatchCVError> {
        // 1️⃣ Fetch CV by ID
        let cv = self
            .repository
            .fetch_cv_by_id(cv_id)
            .await
            .map_err(|err| match err {
                CVRepositoryError::DatabaseError(msg) => PatchCVError::RepositoryError(msg),
                _ => PatchCVError::RepositoryError("Unknown repository error".to_string()),
            })?;

        let existing = match cv {
            Some(cv) => cv,
            None => return Err(PatchCVError::CVNotFound),
        };

        if existing.user_id != user_id {
            return Err(PatchCVError::CVNotFound);
        }

        // 3️⃣ Merge PATCH onto existing state
        let merged = UpdateCVData {
            bio: data.bio.unwrap_or(existing.bio),
            role: data.role.unwrap_or(existing.role),
            display_name: data.display_name.unwrap_or(existing.display_name),
            photo_url: data.photo_url.unwrap_or(existing.photo_url),

            core_skills: data.core_skills.unwrap_or(existing.core_skills),
            educations: data.educations.unwrap_or(existing.educations),
            experiences: data.experiences.unwrap_or(existing.experiences),
            highlighted_projects: data
                .highlighted_projects
                .unwrap_or(existing.highlighted_projects),
            contact_info: data.contact_info.unwrap_or(existing.contact_info),
        };

        // 4️⃣ Delegate to existing update logic
        self.repository
            .update_cv(cv_id, merged)
            .await
            .map_err(|err| match err {
                CVRepositoryError::NotFound => PatchCVError::CVNotFound,
                CVRepositoryError::DatabaseError(msg) => PatchCVError::RepositoryError(msg),
            })
    }
}

#[cfg(test)]
mod patch_tests {
    use std::vec;

    use super::*;
    use crate::cv::application::ports::outgoing::{
        CVRepository, CVRepositoryError, CreateCVData, PatchCVData,
    };
    use crate::cv::domain::entities::CVInfo;
    use async_trait::async_trait;
    use tokio;
    use uuid::Uuid;

    // Reuse the same mock repository pattern
    #[derive(Default)]
    struct MockCVRepository {
        pub existing_cvs: Vec<CVInfo>,
        pub should_fail_update: bool,
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
                display_name: cv_data.display_name,
                role: cv_data.role,
                bio: cv_data.bio,
                photo_url: cv_data.photo_url,
                core_skills: cv_data.core_skills,
                educations: cv_data.educations,
                experiences: cv_data.experiences,
                highlighted_projects: cv_data.highlighted_projects,
                contact_info: cv_data.contact_info,
            })
        }

        async fn create_cv(
            &self,
            _user_id: Uuid,
            _cv_data: CreateCVData,
        ) -> Result<CVInfo, CVRepositoryError> {
            unimplemented!("Create is not used in PatchCV tests")
        }
    }

    // ===========================
    // PATCH SUCCESS
    // ===========================
    #[tokio::test]
    async fn test_patch_cv_success_partial_update() {
        let cv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let existing_cv = CVInfo {
            id: cv_id,
            user_id,
            display_name: "Robin Hood".to_string(),
            role: "Software Engineer".to_string(),
            bio: "Old bio".to_string(),
            photo_url: "https://example.com/old.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        };

        let mock_repo = MockCVRepository {
            existing_cvs: vec![existing_cv.clone()],
            should_fail_update: false,
        };

        let use_case = PatchCVUseCase::new(mock_repo);

        let patch_data = PatchCVData {
            bio: Some("Patched bio".to_string()),
            display_name: Some("Gandalf in The Wood".to_string()),
            role: None,
            photo_url: None,
            core_skills: None,
            educations: None,
            experiences: None,
            highlighted_projects: None,
            contact_info: None,
        };

        let result = use_case.execute(user_id, cv_id, patch_data).await;

        assert!(result.is_ok());
        let updated = result.unwrap();

        // Changed field
        assert_eq!(updated.bio, "Patched bio");
        assert_eq!(updated.display_name, "Gandalf in The Wood");

        // Unchanged fields
        assert_eq!(updated.role, "Software Engineer");
        assert_eq!(updated.photo_url, "https://example.com/old.jpg");
    }

    // ===========================
    // PATCH NOT FOUND
    // ===========================
    #[tokio::test]
    async fn test_patch_cv_not_found() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let mock_repo = MockCVRepository {
            existing_cvs: vec![],
            should_fail_update: false,
        };

        let use_case = PatchCVUseCase::new(mock_repo);

        let patch_data = PatchCVData {
            bio: Some("Patched bio".to_string()),
            display_name: None,
            role: None,
            photo_url: None,
            core_skills: None,
            educations: None,
            experiences: None,
            highlighted_projects: None,
            contact_info: None,
        };

        let result = use_case.execute(user_id, cv_id, patch_data).await;

        match result {
            Err(PatchCVError::CVNotFound) => (),
            other => panic!("Expected CVNotFound, got {:?}", other),
        }
    }

    // ===========================
    // PATCH OWNERSHIP VIOLATION
    // ===========================
    #[tokio::test]
    async fn test_patch_cv_wrong_user() {
        let cv_id = Uuid::new_v4();

        let existing_cv = CVInfo {
            id: cv_id,
            display_name: "Rob Stark".to_string(),
            user_id: Uuid::new_v4(), // different owner
            role: "Software Engineer".to_string(),
            bio: "Old bio".to_string(),
            photo_url: "https://example.com/old.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        };

        let mock_repo = MockCVRepository {
            existing_cvs: vec![existing_cv],
            should_fail_update: false,
        };

        let use_case = PatchCVUseCase::new(mock_repo);

        let patch_data = PatchCVData {
            bio: Some("Hacked bio".to_string()),
            display_name: None,
            role: None,
            photo_url: None,
            core_skills: None,
            educations: None,
            experiences: None,
            highlighted_projects: None,
            contact_info: None,
        };

        let result = use_case.execute(Uuid::new_v4(), cv_id, patch_data).await;

        // IMPORTANT: do not leak existence
        match result {
            Err(PatchCVError::CVNotFound) => (),
            other => panic!("Expected CVNotFound, got {:?}", other),
        }
    }

    // ===========================
    // PATCH DB ERROR
    // ===========================
    #[tokio::test]
    async fn test_patch_cv_db_error() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let existing_cv = CVInfo {
            id: cv_id,
            display_name: "Rob Stark".to_string(),
            user_id: user_id,
            role: "Software Engineer".to_string(),
            bio: "Old bio".to_string(),
            photo_url: "https://example.com/old.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        };

        let mock_repo = MockCVRepository {
            existing_cvs: vec![existing_cv],
            should_fail_update: true,
        };

        let use_case = PatchCVUseCase::new(mock_repo);

        let patch_data = PatchCVData {
            bio: Some("Hacked bio".to_string()),
            display_name: None,
            role: None,
            photo_url: None,
            core_skills: None,
            educations: None,
            experiences: None,
            highlighted_projects: None,
            contact_info: None,
        };

        let result = use_case.execute(user_id, cv_id, patch_data).await;

        match result {
            Err(PatchCVError::RepositoryError(msg)) => {
                assert_eq!(msg, "Update failed");
            }
            other => panic!("Expected RepositoryError, got {:?}", other),
        }
    }
}
