use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};
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
    db: DatabaseConnection,
}

impl CVRepoPostgres {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl CVRepository for CVRepoPostgres {
    async fn fetch_cv_by_user_id(&self, user_id: Uuid) -> Result<CVInfo, CVRepositoryError> {
        let result: Option<CvModel> = CvEntity::find()
            .filter(CvColumn::UserId.eq(user_id))
            .one(&self.db)
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
            bio: Set(cv_data.bio.clone()),
            photo_url: Set(cv_data.photo_url.clone()),
            educations_json: Set(serde_json::to_value(&cv_data.educations)
                .map_err(|e| CVRepositoryError::DatabaseError(e.to_string()))?),
            experiences_json: Set(serde_json::to_value(&cv_data.experiences)
                .map_err(|e| CVRepositoryError::DatabaseError(e.to_string()))?),
            highlighted_projects_json: Set(serde_json::to_value(&cv_data.highlighted_projects)
                .map_err(|e| CVRepositoryError::DatabaseError(e.to_string()))?),
            // set any timestamps or other columns...
            ..Default::default()
        };

        let _ = CvEntity::insert(active)
            .exec(&self.db)
            .await
            .map_err(|err| CVRepositoryError::DatabaseError(err.to_string()))?;

        Ok(cv_data)
    }
    async fn update_cv(&self, user_id: Uuid, cv_data: CVInfo) -> Result<CVInfo, CVRepositoryError> {
        // Find existing CV
        let existing_model = CvEntity::find()
            .filter(CvColumn::UserId.eq(user_id))
            .one(&self.db)
            .await
            .map_err(|err| CVRepositoryError::DatabaseError(err.to_string()))?
            .ok_or(CVRepositoryError::NotFound)?;

        // Convert to ActiveModel and update fields
        let mut active_model = existing_model.into_active_model();

        active_model.bio = Set(cv_data.bio.clone());
        active_model.photo_url = Set(cv_data.photo_url.clone());

        active_model.educations_json = Set(serde_json::to_value(&cv_data.educations)
            .map_err(|e| CVRepositoryError::DatabaseError(e.to_string()))?);

        active_model.experiences_json = Set(serde_json::to_value(&cv_data.experiences)
            .map_err(|e| CVRepositoryError::DatabaseError(e.to_string()))?);

        active_model.highlighted_projects_json =
            Set(serde_json::to_value(&cv_data.highlighted_projects)
                .map_err(|e| CVRepositoryError::DatabaseError(e.to_string()))?);

        // Update timestamp
        active_model.updated_at = Set(chrono::Utc::now().fixed_offset());

        // Perform update
        let updated_model = active_model
            .update(&self.db)
            .await
            .map_err(|err| CVRepositoryError::DatabaseError(err.to_string()))?;

        // Convert to domain model
        Ok(updated_model.to_domain())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cv::application::ports::outgoing::{CVRepository, CVRepositoryError};
    use crate::cv::domain::entities::{CVInfo, Education, Experience, HighlightedProject};
    use ::sea_orm::prelude::*;
    use ::sea_orm::{Database, DatabaseConnection, Statement};
    use dotenvy::dotenv;
    use migration::{Migrator, MigratorTrait};
    use tokio;
    use uuid::Uuid;

    // Helper: Connect to the test database.
    async fn setup_test_db() -> DatabaseConnection {
        dotenv().ok();
        let db_url =
            std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set for tests");
        let conn = Database::connect(&db_url)
            .await
            .expect("Failed to connect to test database");
        // Run migrations so that the cv table is created.
        Migrator::up(&conn, None).await.expect("Migration failed");
        conn
    }

    // Helper: Truncate the 'cv' table to start fresh.
    async fn truncate_cv_table(conn: &DatabaseConnection) {
        // Using raw SQL to truncate the table
        let stmt = Statement::from_string(
            conn.get_database_backend(),
            "TRUNCATE TABLE cv RESTART IDENTITY CASCADE".to_owned(),
        );
        match conn.execute(stmt).await {
            Ok(_) => {}
            Err(err) => {
                let err_str = err.to_string();
                println!("Error:{:?}", &err);
                if err_str.contains("relation \"cv\" does not exist") {
                    println!("Table cv does not exist, skipping truncation.");
                } else {
                    panic!("Failed to truncate cv table: {:?}", err);
                }
            }
        }
    }

    // Helper: Create a sample CVInfo for testing.
    fn sample_cv_info(bio: &str, photo_url: &str) -> CVInfo {
        CVInfo {
            bio: bio.to_owned(),
            photo_url: photo_url.to_owned(),
            educations: vec![Education {
                degree: "B.Sc. Computer Science".to_owned(),
                institution: "Tech University".to_owned(),
                graduation_year: 2020,
            }],
            experiences: vec![Experience {
                company: "Acme Corp".to_owned(),
                position: "Developer".to_owned(),
                start_date: "2020-01-01".to_owned(),
                end_date: None,
                description: "Did some work".to_owned(),
            }],
            highlighted_projects: vec![HighlightedProject {
                id: "proj1".to_owned(),
                title: "Project One".to_owned(),
                slug: "project-one".to_owned(),
                short_description: "Short description".to_owned(),
            }],
        }
    }

    // Test: Create a new CV, then fetch it.
    #[tokio::test]
    async fn test_create_and_fetch_cv() {
        let conn = setup_test_db().await;
        let repo = CVRepoPostgres::new(conn.clone());
        truncate_cv_table(&conn).await;
        let user_id = Uuid::new_v4();
        let cv_data = sample_cv_info("Test bio", "https://example.com/test.jpg");

        // Act: Create the CV.
        let create_result = repo.create_cv(user_id, cv_data.clone()).await;
        assert!(create_result.is_ok(), "Expected CV creation to succeed");
        let created_cv = create_result.unwrap();
        assert_eq!(created_cv.bio, "Test bio");
        assert_eq!(created_cv.photo_url, "https://example.com/test.jpg");

        // Act: Fetch the same CV.
        let fetch_result = repo.fetch_cv_by_user_id(user_id).await;
        assert!(fetch_result.is_ok(), "Expected CV fetch to succeed");
        let fetched_cv = fetch_result.unwrap();
        assert_eq!(fetched_cv.bio, "Test bio");

        // Clean up.
        truncate_cv_table(&conn).await;
    }

    // Test: Create CV duplicate handling.
    #[tokio::test]
    async fn test_create_cv_duplicate() {
        let conn = setup_test_db().await;
        truncate_cv_table(&conn).await;
        let repo = CVRepoPostgres::new(conn.clone());
        let user_id = Uuid::new_v4();
        let cv_data = sample_cv_info("Duplicate bio", "https://example.com/duplicate.jpg");

        // First creation should succeed.
        let res1 = repo.create_cv(user_id, cv_data.clone()).await;
        assert!(res1.is_ok(), "First CV creation should succeed");

        // Second creation for the same user_id should fail (simulate duplicate key).
        let res2 = repo.create_cv(user_id, cv_data).await;
        assert!(res2.is_err(), "Duplicate CV creation should fail");
        match res2 {
            Err(CVRepositoryError::DatabaseError(msg)) => {
                assert!(
                    msg.to_lowercase().contains("duplicate")
                        || msg.to_lowercase().contains("already exists"),
                    "Error message should indicate duplicate key"
                );
            }
            _ => panic!("Expected a DatabaseError for duplicate key"),
        }

        truncate_cv_table(&conn).await;
    }

    // Test: Fetch CV when no record exists.
    #[tokio::test]
    async fn test_fetch_cv_not_found() {
        let conn = setup_test_db().await;
        truncate_cv_table(&conn).await;
        let repo = CVRepoPostgres::new(conn.clone());
        let user_id = Uuid::new_v4();

        let result = repo.fetch_cv_by_user_id(user_id).await;
        match result {
            Err(CVRepositoryError::NotFound) => (),
            _ => panic!("Expected NotFound error when no CV exists"),
        }

        truncate_cv_table(&conn).await;
    }

    // Test: Update an existing CV successfully.
    #[tokio::test]
    async fn test_update_cv_success() {
        let conn = setup_test_db().await;
        truncate_cv_table(&conn).await;
        let repo = CVRepoPostgres::new(conn.clone());
        let user_id = Uuid::new_v4();
        let initial_cv = sample_cv_info("Initial bio", "https://example.com/initial.jpg");

        // Create the CV.
        let _ = repo
            .create_cv(user_id, initial_cv)
            .await
            .expect("CV creation failed");

        // Prepare updated data.
        let updated_cv = sample_cv_info("Updated bio", "https://example.com/updated.jpg");

        // Act: Update the CV.
        let result = repo.update_cv(user_id, updated_cv.clone()).await;
        assert!(result.is_ok(), "Expected update_cv to succeed");
        let updated_result = result.unwrap();
        assert_eq!(updated_result.bio, "Updated bio");
        assert_eq!(updated_result.photo_url, "https://example.com/updated.jpg");

        // Fetch to confirm.
        let fetched = repo.fetch_cv_by_user_id(user_id).await;
        assert!(fetched.is_ok(), "Expected fetch after update to succeed");
        let fetched_cv = fetched.unwrap();
        assert_eq!(fetched_cv.bio, "Updated bio");

        truncate_cv_table(&conn).await;
    }

    // Test: Update CV for a non-existent user.
    #[tokio::test]
    async fn test_update_cv_not_found() {
        let conn = setup_test_db().await;
        truncate_cv_table(&conn).await;
        let repo = CVRepoPostgres::new(conn.clone());
        let user_id = Uuid::new_v4();
        let updated_cv = sample_cv_info("Updated bio", "https://example.com/updated.jpg");

        let result = repo.update_cv(user_id, updated_cv).await;
        match result {
            Err(CVRepositoryError::NotFound) => (),
            _ => panic!("Expected NotFound error when updating non-existent CV"),
        }

        truncate_cv_table(&conn).await;
    }

    // Test: Simulate a DB error during update by dropping the table.
    #[tokio::test]
    async fn test_update_cv_db_error() {
        let conn = setup_test_db().await;
        truncate_cv_table(&conn).await;
        let repo = CVRepoPostgres::new(conn.clone());
        let user_id = Uuid::new_v4();
        let initial_cv = sample_cv_info("Initial bio", "https://example.com/initial.jpg");

        // Create a CV first.
        repo.create_cv(user_id, initial_cv)
            .await
            .expect("CV creation failed");

        // Simulate DB error: rename the table instead of dropping it.
        let rename_result = conn
            .execute(Statement::from_string(
                conn.get_database_backend(),
                "ALTER TABLE cv RENAME TO cv_backup".to_owned(),
            ))
            .await;
        assert!(
            rename_result.is_ok(),
            "Expected table to be renamed for simulation"
        );

        let updated_cv = sample_cv_info("Updated bio", "https://example.com/updated.jpg");
        let result = repo.update_cv(user_id, updated_cv).await;
        match result {
            Err(CVRepositoryError::DatabaseError(msg)) => {
                assert!(
                    msg.to_lowercase().contains("does not exist")
                        || msg.to_lowercase().contains("relation")
                        || msg.to_lowercase().contains("not found"),
                    "Expected error message about missing table, got: {}",
                    msg
                );
            }
            _ => panic!("Expected DatabaseError due to missing table"),
        }

        // Restore the table by renaming it back.
        let restore_result = conn
            .execute(Statement::from_string(
                conn.get_database_backend(),
                "ALTER TABLE cv_backup RENAME TO cv".to_owned(),
            ))
            .await;
        assert!(
            restore_result.is_ok(),
            "Failed to restore cv table after simulation"
        );

        truncate_cv_table(&conn).await;
    }
}
