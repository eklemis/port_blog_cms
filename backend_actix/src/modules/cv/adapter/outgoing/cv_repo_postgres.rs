use crate::cv::application::ports::outgoing::{
    CVRepository, CVRepositoryError, CreateCVData, UpdateCVData,
};
use crate::cv::domain::entities::CVInfo;
use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::sync::Arc;
use uuid::Uuid;

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
    async fn fetch_cv_by_user_id(&self, user_id: Uuid) -> Result<Vec<CVInfo>, CVRepositoryError> {
        let models: Vec<CvModel> = CvEntity::find()
            .filter(CvColumn::UserId.eq(user_id))
            .all(&*self.db)
            .await
            .map_err(|err| CVRepositoryError::DatabaseError(err.to_string()))?;

        Ok(models.into_iter().map(|m| m.to_domain()).collect())
    }

    async fn fetch_cv_by_id(&self, cv_id: Uuid) -> Result<Option<CVInfo>, CVRepositoryError> {
        let model: Option<CvModel> = CvEntity::find_by_id(cv_id)
            .one(&*self.db)
            .await
            .map_err(|err| CVRepositoryError::DatabaseError(err.to_string()))?;

        Ok(model.map(|m| m.to_domain()))
    }

    async fn create_cv(
        &self,
        user_id: Uuid,
        cv_data: CreateCVData,
    ) -> Result<CVInfo, CVRepositoryError> {
        let model = CvModel::from_create_data(user_id, &cv_data);

        let active_model: CvActiveModel = model.into();

        let inserted: CvModel = CvEntity::insert(active_model)
            .exec_with_returning(&*self.db)
            .await
            .map_err(|err| CVRepositoryError::DatabaseError(err.to_string()))?;

        Ok(inserted.to_domain())
    }

    async fn update_cv(
        &self,
        cv_id: Uuid,
        cv_data: UpdateCVData,
    ) -> Result<CVInfo, CVRepositoryError> {
        let active_model = CvActiveModel {
            id: Set(cv_id),
            role: Set(cv_data.role),
            bio: Set(cv_data.bio),
            display_name: Set(cv_data.display_name),
            photo_url: Set(cv_data.photo_url),
            core_skills: Set(serde_json::to_value(&cv_data.core_skills).unwrap()),
            educations: Set(serde_json::to_value(&cv_data.educations).unwrap()),
            experiences: Set(serde_json::to_value(&cv_data.experiences).unwrap()),
            highlighted_projects: Set(serde_json::to_value(&cv_data.highlighted_projects).unwrap()),
            contact_info: Set(serde_json::to_value(&cv_data.contact_info).unwrap()), // ← Add this
            updated_at: Set(chrono::Utc::now().into()),
            ..Default::default()
        };

        let updated = active_model.update(&*self.db).await.map_err(|err| {
            let err_msg = err.to_string();
            // Check if it's a "record not found" error
            if err_msg.contains("None of the records are updated")
                || err_msg.contains("RecordNotUpdated")
            {
                CVRepositoryError::NotFound
            } else {
                CVRepositoryError::DatabaseError(err_msg)
            }
        })?;

        Ok(updated.to_domain())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cv::domain::entities::{
        ContactDetail, ContactType, CoreSkill, Education, Experience, HighlightedProject,
    };
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};

    // Helper function to create a test CV model
    fn create_test_cv_model(user_id: Uuid) -> CvModel {
        let now = Utc::now();
        let fixed_offset_now = now.fixed_offset();

        CvModel {
            id: Uuid::new_v4(), // Add id field
            user_id,
            bio: "Test bio".to_string(),
            display_name: "Robin Hood".to_string(),
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
            contact_info: serde_json::to_value(vec![
                ContactDetail {
                    contact_type: crate::cv::domain::entities::ContactType::PhoneNumber,
                    title: "Personal".to_string(),
                    content: "0876352718".to_string(),
                },
                ContactDetail {
                    contact_type: ContactType::WebPage,
                    title: "Github".to_string(),
                    content: "www.github.com/3423423423kmfdfd".to_string(),
                },
            ])
            .unwrap(),
            created_at: fixed_offset_now,
            updated_at: fixed_offset_now,
            is_deleted: false,
        }
    }

    #[tokio::test]
    async fn test_fetch_cv_by_user_id_found() {
        // Arrange
        let user_id = Uuid::new_v4(); // Convert to String for domain layer
        let cv_model = create_test_cv_model(user_id);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![cv_model.clone()]])
            .into_connection();

        let repo = CVRepoPostgres::new(Arc::new(db));

        // Act
        let result = repo.fetch_cv_by_user_id(user_id).await;

        // Assert
        assert!(result.is_ok(), "Expected fetch_cv_by_user_id to succeed");
        let cv_infos = result.unwrap();

        // Since the method returns Vec<CVInfo>, we need to check the vector
        assert_eq!(cv_infos.len(), 1, "Expected exactly one CV");

        let cv_info = &cv_infos[0];
        assert_eq!(cv_info.id, cv_model.id); // Assert ID is returned correctly
        assert_eq!(cv_info.bio, "Test bio");
        assert_eq!(cv_info.role, "Test role");
        assert_eq!(cv_info.display_name, "Robin Hood");
        assert_eq!(cv_info.photo_url, "https://example.com/photo.jpg");
        assert_eq!(cv_info.core_skills.len(), 1);
        assert_eq!(cv_info.core_skills[0].title, "Rust");
        assert_eq!(cv_info.educations.len(), 1);
        assert_eq!(cv_info.educations[0].degree, "B.Sc. Computer Science");
        assert_eq!(cv_info.experiences.len(), 1);
        assert_eq!(cv_info.experiences[0].company, "Test Corp");
        assert_eq!(cv_info.highlighted_projects.len(), 1);
        assert_eq!(cv_info.highlighted_projects[0].title, "Test Project");
        assert_eq!(cv_info.contact_info.len(), 2);
        assert_eq!(cv_info.contact_info[0].title, "Personal");
        assert_eq!(
            cv_info.contact_info[0].contact_type,
            ContactType::PhoneNumber
        );
        assert_eq!(cv_info.contact_info[0].content, "0876352718");
    }

    #[tokio::test]
    async fn test_fetch_cv_by_user_id_multiple_cvs() {
        // Arrange
        let user_id = Uuid::new_v4();

        let cv_model_1 = create_test_cv_model(user_id);
        let cv_model_2 = create_test_cv_model(user_id);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![cv_model_1.clone(), cv_model_2.clone()]])
            .into_connection();

        let repo = CVRepoPostgres::new(Arc::new(db));

        // Act
        let result = repo.fetch_cv_by_user_id(user_id).await;

        // Assert
        assert!(result.is_ok(), "Expected fetch_cv_by_user_id to succeed");
        let cv_infos = result.unwrap();
        assert_eq!(cv_infos.len(), 2, "Expected two CVs for the user");
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
        assert!(result.is_ok(), "Expected Ok result, got {:?}", result);

        let cvs = result.unwrap();
        assert!(cvs.is_empty(), "Expected empty CV list, got {:?}", cvs);
    }

    #[tokio::test]
    async fn test_create_cv_success() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        // Use CreateCVData instead of CVInfo
        let cv_data = CreateCVData {
            bio: "Test bio".to_string(),
            role: "Test role".to_string(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            display_name: "Robin Hood".to_string(),
            core_skills: vec![CoreSkill {
                title: "Rust".to_string(),
                description: "System programming".to_string(),
            }],
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
            contact_info: vec![
                ContactDetail {
                    contact_type: crate::cv::domain::entities::ContactType::PhoneNumber,
                    title: "Personal".to_string(),
                    content: "0876352718".to_string(),
                },
                ContactDetail {
                    contact_type: ContactType::WebPage,
                    title: "Github".to_string(),
                    content: "www.github.com/3423423423kmfdfd".to_string(),
                },
            ],
        };

        // Create a model that would be returned after insert
        let now = Utc::now();
        let fixed_offset_now = now.fixed_offset();
        let inserted_model = CvModel {
            id: cv_id,
            user_id: user_id,
            display_name: cv_data.display_name.clone(),
            bio: cv_data.bio.clone(),
            role: cv_data.role.clone(),
            photo_url: cv_data.photo_url.clone(),
            core_skills: serde_json::to_value(&cv_data.core_skills).unwrap(),
            educations: serde_json::to_value(&cv_data.educations).unwrap(),
            experiences: serde_json::to_value(&cv_data.experiences).unwrap(),
            highlighted_projects: serde_json::to_value(&cv_data.highlighted_projects).unwrap(),
            contact_info: serde_json::to_value(&cv_data.contact_info).unwrap(),
            created_at: fixed_offset_now,
            updated_at: fixed_offset_now,
            is_deleted: false,
        };

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![inserted_model]])
            .into_connection();

        let repo = CVRepoPostgres::new(Arc::new(db));

        // Act
        let result = repo.create_cv(user_id, cv_data.clone()).await;

        // Assert
        assert!(
            result.is_ok(),
            "Expected create_cv to succeed, got {:?}",
            result
        );
        let created_cv = result.unwrap();

        // Verify the ID is returned and matches
        assert_eq!(created_cv.id, cv_id);
        assert_eq!(created_cv.bio, cv_data.bio);
        assert_eq!(created_cv.role, cv_data.role);
        assert_eq!(created_cv.photo_url, cv_data.photo_url);
        assert_eq!(created_cv.core_skills.len(), cv_data.core_skills.len());
        assert_eq!(created_cv.educations.len(), cv_data.educations.len());
        assert_eq!(created_cv.experiences.len(), cv_data.experiences.len());
        assert_eq!(
            created_cv.highlighted_projects.len(),
            cv_data.highlighted_projects.len()
        );
    }

    #[tokio::test]
    async fn test_update_cv_success() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let updated_cv_data = UpdateCVData {
            bio: "Updated bio".to_string(),
            role: "Updated role".to_string(),
            display_name: "Robin Hood".to_string(),
            photo_url: "https://example.com/updated.jpg".to_string(),
            core_skills: vec![CoreSkill {
                title: "Advanced Rust".to_string(),
                description: "Expert level".to_string(),
            }],
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
            contact_info: vec![
                ContactDetail {
                    contact_type: crate::cv::domain::entities::ContactType::PhoneNumber,
                    title: "Personal".to_string(),
                    content: "0876352718".to_string(),
                },
                ContactDetail {
                    contact_type: ContactType::WebPage,
                    title: "Github".to_string(),
                    content: "www.github.com/3423423423kmfdfd".to_string(),
                },
            ],
        };

        // Build the expected result model directly - NO cloning from existing_cv_model
        let now = Utc::now().fixed_offset();
        let updated_model = CvModel {
            id: cv_id,
            user_id,
            bio: "Updated bio".to_string(), // ← Direct value, not from clone
            display_name: "Robin Hood".to_string(),
            role: "Updated role".to_string(),
            photo_url: "https://example.com/updated.jpg".to_string(),
            core_skills: serde_json::to_value(&updated_cv_data.core_skills).unwrap(),
            educations: serde_json::to_value(&updated_cv_data.educations).unwrap(),
            experiences: serde_json::to_value(&updated_cv_data.experiences).unwrap(),
            highlighted_projects: serde_json::to_value(&updated_cv_data.highlighted_projects)
                .unwrap(),
            contact_info: serde_json::to_value(&updated_cv_data.contact_info).unwrap(),
            created_at: now,
            updated_at: now,
            is_deleted: false,
        };

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 0,
                rows_affected: 1,
            }])
            .append_query_results(vec![vec![updated_model.clone()]])
            .into_connection();

        let repo = CVRepoPostgres::new(Arc::new(db));

        // Act
        let result = repo.update_cv(cv_id, updated_cv_data).await;

        // Assert
        assert!(
            result.is_ok(),
            "Expected update_cv to succeed, got {:?}",
            result
        );
        let updated_cv = result.unwrap();

        assert_eq!(updated_cv.id, cv_id);
        assert_eq!(updated_cv.bio, "Updated bio");
        assert_eq!(updated_cv.role, "Updated role");
        assert_eq!(updated_cv.photo_url, "https://example.com/updated.jpg");
        assert_eq!(updated_cv.core_skills.len(), 1);
        assert_eq!(updated_cv.core_skills[0].title, "Advanced Rust");
    }

    #[tokio::test]
    async fn test_update_cv_not_found() {
        // Arrange
        let cv_id = Uuid::new_v4(); // Use CV ID as String

        // Use UpdateCVData instead of CVInfo
        let cv_data = UpdateCVData {
            bio: "Updated bio".to_string(),
            role: "Updated role".to_string(),
            display_name: "Robin Hood".to_string(),
            photo_url: "https://example.com/updated.jpg".to_string(),
            core_skills: vec![],
            educations: vec![Education {
                degree: "M.Sc. Computer Science".to_string(),
                institution: "Advanced University".to_string(),
                graduation_year: 2022,
            }],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        };

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<CvModel>::new()]) // Empty result - CV not found
            .into_connection();

        let repo = CVRepoPostgres::new(Arc::new(db));

        // Act
        let result = repo.update_cv(cv_id, cv_data).await;

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
