use crate::cv::application::ports::outgoing::{CVRepository, CVRepositoryError, CreateCVData};
use crate::cv::domain::entities::CVInfo;
use async_trait::async_trait;
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum CreateCVError {
    RepositoryError(String),
}

impl fmt::Display for CreateCVError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CreateCVError::RepositoryError(msg) => {
                write!(f, "repository error: {}", msg)
            }
        }
    }
}

/// An interface for the create CV use case
#[async_trait]
pub trait ICreateCVUseCase: Send + Sync {
    async fn execute(&self, user_id: Uuid, cv_data: CreateCVData) -> Result<CVInfo, CreateCVError>;
}

/// Concrete implementation of the create CV use case
pub struct CreateCVUseCase<R>
where
    R: CVRepository,
{
    cv_repository: R,
}

impl<R> CreateCVUseCase<R>
where
    R: CVRepository,
{
    pub fn new(cv_repository: R) -> Self {
        Self { cv_repository }
    }
}

#[async_trait]
impl<R> ICreateCVUseCase for CreateCVUseCase<R>
where
    R: CVRepository + Sync + Send,
{
    async fn execute(&self, user_id: Uuid, cv_data: CreateCVData) -> Result<CVInfo, CreateCVError> {
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
    use crate::cv::domain::entities::{CVInfo, CoreSkill};
    use async_trait::async_trait;
    use uuid::Uuid;

    // ========================================================================
    // Mock Implementations
    // ========================================================================

    #[derive(Clone)]
    struct MockCVRepository {
        create_result: Result<CVInfo, CVRepositoryError>,
        fetch_by_user_result: Result<Vec<CVInfo>, CVRepositoryError>,
    }

    impl MockCVRepository {
        fn new() -> Self {
            Self {
                create_result: Ok(Self::default_cv_info()),
                fetch_by_user_result: Ok(vec![]),
            }
        }

        fn with_create_error(mut self, error: CVRepositoryError) -> Self {
            self.create_result = Err(error);
            self
        }

        fn default_cv_info() -> CVInfo {
            CVInfo {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                display_name: "Test User".to_string(),
                role: "Software Engineer".to_string(),
                bio: "Test bio".to_string(),
                photo_url: "https://example.com/photo.jpg".to_string(),
                core_skills: vec![],
                educations: vec![],
                experiences: vec![],
                highlighted_projects: vec![],
                contact_info: vec![],
            }
        }
    }

    #[async_trait]
    impl CVRepository for MockCVRepository {
        async fn fetch_cv_by_user_id(
            &self,
            _user_id: Uuid,
        ) -> Result<Vec<CVInfo>, CVRepositoryError> {
            self.fetch_by_user_result.clone()
        }

        async fn fetch_cv_by_id(&self, _cv_id: Uuid) -> Result<Option<CVInfo>, CVRepositoryError> {
            Ok(None)
        }

        async fn create_cv(
            &self,
            user_id: Uuid,
            cv_data: CreateCVData,
        ) -> Result<CVInfo, CVRepositoryError> {
            match &self.create_result {
                Ok(_) => Ok(CVInfo {
                    id: Uuid::new_v4(),
                    user_id,
                    display_name: cv_data.display_name,
                    role: cv_data.role,
                    bio: cv_data.bio,
                    photo_url: cv_data.photo_url,
                    core_skills: cv_data.core_skills,
                    educations: cv_data.educations,
                    experiences: cv_data.experiences,
                    highlighted_projects: cv_data.highlighted_projects,
                    contact_info: cv_data.contact_info,
                }),
                Err(e) => Err(e.clone()),
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

    // ========================================================================
    // Helper Functions
    // ========================================================================

    fn create_valid_cv_data() -> CreateCVData {
        CreateCVData {
            display_name: "John Doe".to_string(),
            role: "Software Engineer".to_string(),
            bio: "Experienced software engineer with passion for clean code".to_string(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        }
    }

    fn create_use_case() -> CreateCVUseCase<MockCVRepository> {
        CreateCVUseCase::new(MockCVRepository::new())
    }

    // ========================================================================
    // Success Cases
    // ========================================================================

    #[tokio::test]
    async fn test_create_cv_success() {
        let use_case = create_use_case();
        let user_id = Uuid::new_v4();
        let cv_data = create_valid_cv_data();

        let result = use_case.execute(user_id, cv_data.clone()).await;

        assert!(result.is_ok());
        let created_cv = result.unwrap();
        assert_eq!(created_cv.display_name, cv_data.display_name);
        assert_eq!(created_cv.role, cv_data.role);
        assert_eq!(created_cv.bio, cv_data.bio);
        assert_eq!(created_cv.photo_url, cv_data.photo_url);
        assert_eq!(created_cv.user_id, user_id);
    }

    #[tokio::test]
    async fn test_create_cv_with_all_fields_populated() {
        let use_case = create_use_case();
        let user_id = Uuid::new_v4();

        let core_skills = vec![
            CoreSkill {
                title: "Rust".to_string(),
                description: "Systems programming language with focus on safety and performance"
                    .to_string(),
            },
            CoreSkill {
                title: "Python".to_string(),
                description: "High-level programming language for rapid development".to_string(),
            },
        ];

        let cv_data = CreateCVData {
            display_name: "Jane Smith".to_string(),
            role: "Senior Developer".to_string(),
            bio: "10 years of experience in web development".to_string(),
            photo_url: "https://example.com/jane.jpg".to_string(),
            core_skills: core_skills.clone(),
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        };

        let result = use_case.execute(user_id, cv_data.clone()).await;

        assert!(result.is_ok());
        let created_cv = result.unwrap();
        assert_eq!(created_cv.core_skills.len(), 2);
        assert_eq!(created_cv.core_skills[0].title, "Rust");
        assert_eq!(
            created_cv.core_skills[0].description,
            "Systems programming language with focus on safety and performance"
        );
        assert_eq!(created_cv.core_skills[1].title, "Python");
    }

    #[tokio::test]
    async fn test_create_cv_with_minimal_data() {
        let use_case = create_use_case();
        let user_id = Uuid::new_v4();

        let cv_data = CreateCVData {
            display_name: "Min User".to_string(),
            role: "Developer".to_string(),
            bio: "".to_string(),
            photo_url: "".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        };

        let result = use_case.execute(user_id, cv_data).await;

        assert!(result.is_ok());
        let created_cv = result.unwrap();
        assert_eq!(created_cv.display_name, "Min User");
        assert_eq!(created_cv.bio, "");
    }

    // ========================================================================
    // Repository Error Cases
    // ========================================================================

    #[tokio::test]
    async fn test_create_cv_database_error() {
        let repository = MockCVRepository::new().with_create_error(
            CVRepositoryError::DatabaseError("DB insert failed".to_string()),
        );

        let use_case = CreateCVUseCase::new(repository);
        let user_id = Uuid::new_v4();
        let cv_data = create_valid_cv_data();

        let result = use_case.execute(user_id, cv_data).await;

        let CreateCVError::RepositoryError(msg) = result.expect_err("Expected error, got Ok");

        assert_eq!(msg, "DB insert failed");
    }

    #[tokio::test]
    async fn test_create_cv_not_found_error() {
        let repository = MockCVRepository::new().with_create_error(CVRepositoryError::NotFound);

        let use_case = CreateCVUseCase::new(repository);
        let user_id = Uuid::new_v4();
        let cv_data = create_valid_cv_data();

        let result = use_case.execute(user_id, cv_data).await;

        let CreateCVError::RepositoryError(msg) = result.expect_err("Expected error, got Ok");

        assert_eq!(msg, "Unknown repo error");
    }

    #[tokio::test]
    async fn test_create_cv_connection_error() {
        let repository = MockCVRepository::new().with_create_error(
            CVRepositoryError::DatabaseError("Connection timeout".to_string()),
        );

        let use_case = CreateCVUseCase::new(repository);
        let user_id = Uuid::new_v4();
        let cv_data = create_valid_cv_data();

        let result = use_case.execute(user_id, cv_data).await;

        let CreateCVError::RepositoryError(msg) = result.expect_err("Expected error, got Ok");

        assert_eq!(msg, "Connection timeout");
    }

    // ========================================================================
    // Edge Cases
    // ========================================================================

    #[tokio::test]
    async fn test_create_cv_preserves_user_id() {
        let use_case = create_use_case();
        let user_id = Uuid::new_v4();
        let cv_data = create_valid_cv_data();

        let result = use_case.execute(user_id, cv_data).await;

        assert!(result.is_ok());
        let created_cv = result.unwrap();
        assert_eq!(created_cv.user_id, user_id);
    }

    #[tokio::test]
    async fn test_create_cv_generates_unique_id() {
        let use_case = create_use_case();
        let user_id = Uuid::new_v4();
        let cv_data = create_valid_cv_data();

        let result1 = use_case.execute(user_id, cv_data.clone()).await;
        let result2 = use_case.execute(user_id, cv_data).await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let cv1 = result1.unwrap();
        let cv2 = result2.unwrap();

        // Each CV should have a unique ID
        assert_ne!(cv1.id, cv2.id);
    }

    #[tokio::test]
    async fn test_create_cv_with_long_bio() {
        let use_case = create_use_case();
        let user_id = Uuid::new_v4();

        let long_bio = "a".repeat(5000);
        let cv_data = CreateCVData {
            display_name: "Test User".to_string(),
            role: "Developer".to_string(),
            bio: long_bio.clone(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        };

        let result = use_case.execute(user_id, cv_data).await;

        assert!(result.is_ok());
        let created_cv = result.unwrap();
        assert_eq!(created_cv.bio, long_bio);
    }

    #[tokio::test]
    async fn test_create_cv_with_special_characters_in_name() {
        let use_case = create_use_case();
        let user_id = Uuid::new_v4();

        let cv_data = CreateCVData {
            display_name: "José María O'Brien-Smith".to_string(),
            role: "Software Engineer".to_string(),
            bio: "Developer with international experience".to_string(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        };

        let result = use_case.execute(user_id, cv_data.clone()).await;

        assert!(result.is_ok());
        let created_cv = result.unwrap();
        assert_eq!(created_cv.display_name, cv_data.display_name);
    }

    // ========================================================================
    // Error Type Tests
    // ========================================================================

    #[test]
    fn test_error_display() {
        let error = CreateCVError::RepositoryError("Test error".to_string());
        let display = format!("{}", error);
        assert!(!display.is_empty());
        assert!(display.contains("Test error"));
    }

    #[test]
    fn test_error_debug() {
        let error = CreateCVError::RepositoryError("Debug test".to_string());
        let debug = format!("{:?}", error);
        assert!(!debug.is_empty());
    }

    // ========================================================================
    // Integration Tests
    // ========================================================================

    #[tokio::test]
    async fn test_multiple_cv_creation_for_same_user() {
        let use_case = create_use_case();
        let user_id = Uuid::new_v4();

        let cv_data1 = CreateCVData {
            display_name: "First CV".to_string(),
            role: "Junior Developer".to_string(),
            bio: "First version".to_string(),
            photo_url: "https://example.com/photo1.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        };

        let cv_data2 = CreateCVData {
            display_name: "Second CV".to_string(),
            role: "Senior Developer".to_string(),
            bio: "Second version".to_string(),
            photo_url: "https://example.com/photo2.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        };

        let result1 = use_case.execute(user_id, cv_data1).await;
        let result2 = use_case.execute(user_id, cv_data2).await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let cv1 = result1.unwrap();
        let cv2 = result2.unwrap();

        assert_eq!(cv1.user_id, user_id);
        assert_eq!(cv2.user_id, user_id);
        assert_ne!(cv1.id, cv2.id);
        assert_eq!(cv1.role, "Junior Developer");
        assert_eq!(cv2.role, "Senior Developer");
    }

    #[tokio::test]
    async fn test_create_cv_different_users() {
        let use_case = create_use_case();
        let user_id1 = Uuid::new_v4();
        let user_id2 = Uuid::new_v4();
        let cv_data = create_valid_cv_data();

        let result1 = use_case.execute(user_id1, cv_data.clone()).await;
        let result2 = use_case.execute(user_id2, cv_data).await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let cv1 = result1.unwrap();
        let cv2 = result2.unwrap();

        assert_eq!(cv1.user_id, user_id1);
        assert_eq!(cv2.user_id, user_id2);
        assert_ne!(cv1.user_id, cv2.user_id);
    }
}
