use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::cv::application::ports::outgoing::{CVRepository, CVRepositoryError};
use crate::cv::domain::entities::CVInfo;
use async_trait::async_trait;

// Bring in the entity we just defined above:
use super::sea_orm_entity::{
    ActiveModel as CvActiveModel, Column as CvColumn, Entity as CvEntity, Model as CvModel,
};

#[derive(Debug, Clone)]
pub struct CVRepoPostgres {
    db: Arc<DatabaseConnection>,
}

impl CVRepoPostgres {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl CVRepository for CVRepoPostgres {
    async fn fetch_cv_by_user_id(&self, user_id: Uuid) -> Result<CVInfo, CVRepositoryError> {
        let result: Option<CvModel> = CvEntity::find()
            .filter(CvColumn::UserId.eq(user_id))
            .one(&*self.db)
            .await
            .map_err(|err| CVRepositoryError::DatabaseError(err.to_string()))?;

        match result {
            Some(model) => {
                let cv_info = model.to_domain();
                Ok(cv_info)
            }
            None => Err(CVRepositoryError::NotFound),
        }
    }
    async fn create_cv(&self, user_id: Uuid, cv_data: CVInfo) -> Result<CVInfo, CVRepositoryError> {
        // Convert domain CVInfo -> SeaORM ActiveModel
        let active = CvActiveModel {
            user_id: Set(user_id),
            role: Set(cv_data.role.clone()),
            bio: Set(cv_data.bio.clone()),
            photo_url: Set(cv_data.photo_url.clone()),
            core_skills: Set(serde_json::to_value(&cv_data.core_skills)
                .map_err(|e| CVRepositoryError::DatabaseError(e.to_string()))?),
            educations: Set(serde_json::to_value(&cv_data.educations)
                .map_err(|e| CVRepositoryError::DatabaseError(e.to_string()))?),
            experiences: Set(serde_json::to_value(&cv_data.experiences)
                .map_err(|e| CVRepositoryError::DatabaseError(e.to_string()))?),
            highlighted_projects: Set(serde_json::to_value(&cv_data.highlighted_projects)
                .map_err(|e| CVRepositoryError::DatabaseError(e.to_string()))?),
            // set any timestamps or other columns...
            ..Default::default()
        };

        let _ = CvEntity::insert(active)
            .exec(&*self.db)
            .await
            .map_err(|err| CVRepositoryError::DatabaseError(err.to_string()))?;

        Ok(cv_data)
    }
    async fn update_cv(&self, user_id: Uuid, cv_data: CVInfo) -> Result<CVInfo, CVRepositoryError> {
        // Find existing CV
        let existing_model = CvEntity::find()
            .filter(CvColumn::UserId.eq(user_id))
            .one(&*self.db)
            .await
            .map_err(|err| CVRepositoryError::DatabaseError(err.to_string()))?
            .ok_or(CVRepositoryError::NotFound)?;

        // Convert to ActiveModel and update fields
        let mut active_model = existing_model.into_active_model();

        active_model.role = Set(cv_data.role.clone());
        active_model.bio = Set(cv_data.bio.clone());
        active_model.photo_url = Set(cv_data.photo_url.clone());

        active_model.core_skills = Set(serde_json::to_value(&cv_data.core_skills)
            .map_err(|e| CVRepositoryError::DatabaseError((e.to_string())))?);

        active_model.educations = Set(serde_json::to_value(&cv_data.educations)
            .map_err(|e| CVRepositoryError::DatabaseError(e.to_string()))?);

        active_model.experiences = Set(serde_json::to_value(&cv_data.experiences)
            .map_err(|e| CVRepositoryError::DatabaseError(e.to_string()))?);

        active_model.highlighted_projects =
            Set(serde_json::to_value(&cv_data.highlighted_projects)
                .map_err(|e| CVRepositoryError::DatabaseError(e.to_string()))?);

        // Update timestamp
        active_model.updated_at = Set(chrono::Utc::now().fixed_offset());

        // Perform update
        let updated_model = active_model
            .update(&*self.db)
            .await
            .map_err(|err| CVRepositoryError::DatabaseError(err.to_string()))?;

        // Convert to domain model
        Ok(updated_model.to_domain())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cv::domain::entities::{CoreSkill, Education, Experience, HighlightedProject};
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};

    // Helper function to create a test CV model
    fn create_test_cv_model(user_id: Uuid) -> CvModel {
        let now = Utc::now();
        let fixed_offset_now = now.fixed_offset();

        CvModel {
            user_id,
            bio: "Test bio".to_string(),
            role: "Test role".to_string(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            core_skills: serde_json::to_value(vec![CoreSkill {
                title: "Rust".to_string(),
                description: "System programming".to_string(),
            }])
            .unwrap(),
            educations: serde_json::to_value(vec![Education {
                degree: "B.Sc. Computer Science".to_string(),
                institution: "Test University".to_string(),
                graduation_year: 2020,
            }])
            .unwrap(),
            experiences: serde_json::to_value(vec![Experience {
                company: "Test Corp".to_string(),
                position: "Developer".to_string(),
                location: "Jakarta, Indonesia".to_string(),
                start_date: "2020-01-01".to_string(),
                end_date: None,
                description: "Test description".to_string(),
                tasks: vec![],
                achievements: vec![],
            }])
            .unwrap(),
            highlighted_projects: serde_json::to_value(vec![HighlightedProject {
                id: "proj1".to_string(),
                title: "Test Project".to_string(),
                slug: "test-project".to_string(),
                short_description: "Short description".to_string(),
            }])
            .unwrap(),
            created_at: fixed_offset_now,
            updated_at: fixed_offset_now,
        }
    }

    // Helper function to create a test CV info domain object
    fn create_test_cv_info() -> CVInfo {
        CVInfo {
            bio: "Test bio".to_string(),
            role: "Test role".to_string(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            core_skills: vec![],
            educations: vec![Education {
                degree: "B.Sc. Computer Science".to_string(),
                institution: "Test University".to_string(),
                graduation_year: 2020,
            }],
            experiences: vec![Experience {
                company: "Test Corp".to_string(),
                position: "Developer".to_string(),
                location: "Jakarta, Indonesia".to_string(),
                start_date: "2020-01-01".to_string(),
                end_date: None,
                description: "Test description".to_string(),
                tasks: vec![],
                achievements: vec![],
            }],
            highlighted_projects: vec![HighlightedProject {
                id: "proj1".to_string(),
                title: "Test Project".to_string(),
                slug: "test-project".to_string(),
                short_description: "Short description".to_string(),
            }],
        }
    }

    #[tokio::test]
    async fn test_fetch_cv_by_user_id_found() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_model = create_test_cv_model(user_id);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![cv_model.clone()]])
            .into_connection();

        let repo = CVRepoPostgres::new(Arc::new(db));

        // Act
        let result = repo.fetch_cv_by_user_id(user_id).await;

        // Assert
        assert!(result.is_ok(), "Expected fetch_cv_by_user_id to succeed");
        let cv_info = result.unwrap();
        assert_eq!(cv_info.bio, "Test bio");
        assert_eq!(cv_info.photo_url, "https://example.com/photo.jpg");
        assert_eq!(cv_info.educations.len(), 1);
        assert_eq!(cv_info.educations[0].degree, "B.Sc. Computer Science");
        assert_eq!(cv_info.experiences.len(), 1);
        assert_eq!(cv_info.experiences[0].company, "Test Corp");
        assert_eq!(cv_info.highlighted_projects.len(), 1);
        assert_eq!(cv_info.highlighted_projects[0].title, "Test Project");
    }

    #[tokio::test]
    async fn test_fetch_cv_by_user_id_not_found() {
        // Arrange
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<CvModel>::new()]) // Empty result
            .into_connection();

        let repo = CVRepoPostgres::new(Arc::new(db));

        // Act
        let result = repo.fetch_cv_by_user_id(user_id).await;

        // Assert
        assert!(
            matches!(result, Err(CVRepositoryError::NotFound)),
            "Expected NotFound error, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_create_cv_success() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_info = create_test_cv_info();

        // Create a model that would be returned after insert
        let now = Utc::now();
        let fixed_offset_now = now.fixed_offset();
        let inserted_model = CvModel {
            user_id,
            bio: cv_info.bio.clone(),
            role: cv_info.role.clone(),
            photo_url: cv_info.photo_url.clone(),
            core_skills: serde_json::to_value(&cv_info.core_skills).unwrap(),
            educations: serde_json::to_value(&cv_info.educations).unwrap(),
            experiences: serde_json::to_value(&cv_info.experiences).unwrap(),
            highlighted_projects: serde_json::to_value(&cv_info.highlighted_projects).unwrap(),
            created_at: fixed_offset_now,
            updated_at: fixed_offset_now,
        };

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // In SeaORM, the insert operation might need a query result
            // for the inserted model, especially if it's returning the model
            .append_query_results(vec![vec![inserted_model]])
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 1,
                rows_affected: 1,
            }])
            .into_connection();

        let repo = CVRepoPostgres::new(Arc::new(db));

        // Act
        let result = repo.create_cv(user_id, cv_info.clone()).await;

        // Assert
        assert!(
            result.is_ok(),
            "Expected create_cv to succeed, got {:?}",
            result
        );
        let created_cv = result.unwrap();
        assert_eq!(created_cv.bio, cv_info.bio);
        assert_eq!(created_cv.photo_url, cv_info.photo_url);
        assert_eq!(created_cv.educations.len(), cv_info.educations.len());
        assert_eq!(created_cv.experiences.len(), cv_info.experiences.len());
        assert_eq!(
            created_cv.highlighted_projects.len(),
            cv_info.highlighted_projects.len()
        );
    }

    #[tokio::test]
    async fn test_update_cv_success() {
        // Arrange
        let user_id = Uuid::new_v4();
        let existing_cv_model = create_test_cv_model(user_id);
        let updated_cv_info = CVInfo {
            bio: "Updated bio".to_string(),
            role: "Test role".to_string(),
            photo_url: "https://example.com/updated.jpg".to_string(),
            core_skills: vec![],
            educations: vec![Education {
                degree: "M.Sc. Computer Science".to_string(),
                institution: "Advanced University".to_string(),
                graduation_year: 2022,
            }],
            experiences: vec![Experience {
                company: "Advanced Corp".to_string(),
                position: "Senior Developer".to_string(),
                location: "Jakarta, Indonesia".to_string(),
                start_date: "2022-01-01".to_string(),
                end_date: None,
                description: "Advanced work".to_string(),
                tasks: vec![],
                achievements: vec![],
            }],
            highlighted_projects: vec![HighlightedProject {
                id: "proj2".to_string(),
                title: "Advanced Project".to_string(),
                slug: "advanced-project".to_string(),
                short_description: "Advanced description".to_string(),
            }],
        };

        // Create an updated model that will be returned after update
        let mut updated_model = existing_cv_model.clone();
        updated_model.role = "updated role".to_string();
        updated_model.bio = "Updated bio".to_string();
        updated_model.photo_url = "https://example.com/updated.jpg".to_string();
        updated_model.core_skills = serde_json::to_value(&updated_cv_info.core_skills).unwrap();
        updated_model.educations = serde_json::to_value(&updated_cv_info.educations).unwrap();
        updated_model.experiences = serde_json::to_value(&updated_cv_info.experiences).unwrap();
        updated_model.highlighted_projects =
            serde_json::to_value(&updated_cv_info.highlighted_projects).unwrap();
        updated_model.updated_at = Utc::now().fixed_offset();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // First query - find existing CV
            .append_query_results(vec![vec![existing_cv_model]])
            // Second query - return updated model after update
            .append_query_results(vec![vec![updated_model.clone()]])
            // Exec result for the update operation
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 0,
                rows_affected: 1,
            }])
            .into_connection();

        let repo = CVRepoPostgres::new(Arc::new(db));

        // Act
        let result = repo.update_cv(user_id, updated_cv_info.clone()).await;

        // Assert
        assert!(
            result.is_ok(),
            "Expected update_cv to succeed, got {:?}",
            result
        );
        let updated_cv = result.unwrap();
        assert_eq!(updated_cv.bio, "Updated bio");
        assert_eq!(updated_cv.photo_url, "https://example.com/updated.jpg");
        assert_eq!(updated_cv.educations.len(), 1);
        assert_eq!(updated_cv.educations[0].degree, "M.Sc. Computer Science");
        assert_eq!(updated_cv.experiences.len(), 1);
        assert_eq!(updated_cv.experiences[0].company, "Advanced Corp");
        assert_eq!(updated_cv.highlighted_projects.len(), 1);
        assert_eq!(updated_cv.highlighted_projects[0].title, "Advanced Project");
    }

    #[tokio::test]
    async fn test_update_cv_not_found() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_info = create_test_cv_info();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<CvModel>::new()]) // Empty result
            .into_connection();

        let repo = CVRepoPostgres::new(Arc::new(db));

        // Act
        let result = repo.update_cv(user_id, cv_info).await;

        // Assert
        assert!(
            matches!(result, Err(CVRepositoryError::NotFound)),
            "Expected NotFound error, got {:?}",
            result
        );
    }

    #[test]
    fn test_instance_can_be_cloned() {
        // Arrange
        let db = MockDatabase::new(DatabaseBackend::Postgres).into_connection();
        let repo = CVRepoPostgres::new(Arc::new(db));

        // Act
        let _ = repo.clone();

        // Assert - if it compiles, the test passes since Arc is working
        assert!(true);
    }
}
