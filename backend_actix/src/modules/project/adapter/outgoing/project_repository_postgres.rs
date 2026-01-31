use async_trait::async_trait;
use chrono::Utc;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::adapter::outgoing::sea_orm_entity::projects::{
    self, ActiveModel, Column, Entity,
};
use crate::modules::project::application::ports::outgoing::project_repository::{
    CreateProjectData, PatchField, PatchProjectData, ProjectRepository, ProjectRepositoryError,
    ProjectResult,
};

// ============================================================================
// Repository Implementation
// ============================================================================

#[derive(Clone)]
pub struct ProjectRepositoryPostgres {
    db: Arc<DatabaseConnection>,
}

impl ProjectRepositoryPostgres {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ProjectRepository for ProjectRepositoryPostgres {
    async fn create_project(
        &self,
        data: CreateProjectData,
    ) -> Result<ProjectResult, ProjectRepositoryError> {
        let owner_uuid: Uuid = data.owner.into();
        let now = Utc::now().fixed_offset();

        let model = ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(owner_uuid),
            title: Set(data.title.trim().to_string()),
            slug: Set(data.slug.trim().to_lowercase()),
            description: Set(data.description),
            tech_stack: Set(data.tech_stack),
            screenshots: Set(to_json(&data.screenshots)?),
            repo_url: Set(data.repo_url),
            live_demo_url: Set(data.live_demo_url),
            is_deleted: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = model.insert(&*self.db).await.map_err(map_slug_error)?;

        model_to_result(result)
    }

    async fn patch_project(
        &self,
        owner: UserId,
        project_id: Uuid,
        data: PatchProjectData,
    ) -> Result<ProjectResult, ProjectRepositoryError> {
        let owner_uuid: Uuid = owner.into();

        let mut model = <ActiveModel as Default>::default();

        if let PatchField::Value(title) = data.title {
            model.title = Set(title.trim().to_string());
        }

        if let PatchField::Value(desc) = data.description {
            model.description = Set(desc);
        }

        if let PatchField::Value(tech) = data.tech_stack {
            model.tech_stack = Set(tech);
        }

        if let PatchField::Value(screens) = data.screenshots {
            model.screenshots = Set(to_json(&screens)?);
        }

        match data.repo_url {
            PatchField::Unset => {}
            PatchField::Null => model.repo_url = Set(None),
            PatchField::Value(url) => model.repo_url = Set(Some(url)),
        }

        match data.live_demo_url {
            PatchField::Unset => {}
            PatchField::Null => model.live_demo_url = Set(None),
            PatchField::Value(url) => model.live_demo_url = Set(Some(url)),
        }

        let has_changes = model.title.is_set()
            || model.description.is_set()
            || model.tech_stack.is_set()
            || model.screenshots.is_set()
            || model.repo_url.is_set()
            || model.live_demo_url.is_set();

        if !has_changes {
            let result = Entity::find_by_id(project_id)
                .filter(Column::UserId.eq(owner_uuid))
                .filter(Column::IsDeleted.eq(false))
                .one(&*self.db)
                .await
                .map_err(map_db_err)?
                .ok_or(ProjectRepositoryError::NotFound)?;

            return model_to_result(result);
        }

        let results = Entity::update_many()
            .set(model)
            .filter(Column::Id.eq(project_id))
            .filter(Column::UserId.eq(owner_uuid))
            .filter(Column::IsDeleted.eq(false))
            .exec_with_returning(&*self.db)
            .await
            .map_err(map_db_err)?;

        let result = results
            .into_iter()
            .next()
            .ok_or(ProjectRepositoryError::NotFound)?;

        model_to_result(result)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn model_to_result(model: projects::Model) -> Result<ProjectResult, ProjectRepositoryError> {
    Ok(ProjectResult {
        id: model.id,
        owner: UserId::from(model.user_id),
        title: model.title,
        slug: model.slug,
        description: model.description,
        tech_stack: model.tech_stack,
        screenshots: from_json(&model.screenshots)?,
        repo_url: model.repo_url,
        live_demo_url: model.live_demo_url,
        created_at: model.created_at.into(),
        updated_at: model.updated_at.into(),
    })
}

fn to_json<T: serde::Serialize>(data: &T) -> Result<serde_json::Value, ProjectRepositoryError> {
    serde_json::to_value(data)
        .map_err(|e| ProjectRepositoryError::SerializationError(e.to_string()))
}

fn from_json<T: serde::de::DeserializeOwned>(
    json: &serde_json::Value,
) -> Result<T, ProjectRepositoryError> {
    serde_json::from_value(json.clone())
        .map_err(|e| ProjectRepositoryError::SerializationError(e.to_string()))
}

fn map_slug_error(e: DbErr) -> ProjectRepositoryError {
    let msg = e.to_string().to_lowercase();

    if (msg.contains("duplicate") || msg.contains("unique") || msg.contains("23505"))
        && msg.contains("slug")
    {
        ProjectRepositoryError::SlugAlreadyExists
    } else {
        ProjectRepositoryError::DatabaseError(e.to_string())
    }
}
fn map_db_err(e: DbErr) -> ProjectRepositoryError {
    ProjectRepositoryError::DatabaseError(e.to_string())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, DbErr, MockDatabase};
    use uuid::Uuid;

    fn create_test_project_data() -> CreateProjectData {
        CreateProjectData {
            owner: UserId::from(Uuid::new_v4()),
            title: "Test Project".to_string(),
            slug: "test-project".to_string(),
            description: "A test project description".to_string(),
            tech_stack: vec!["Rust".to_string(), "PostgreSQL".to_string()],
            screenshots: vec!["screenshot1.png".to_string()],
            repo_url: Some("https://github.com/user/repo".to_string()),
            live_demo_url: Some("https://demo.example.com".to_string()),
        }
    }

    fn create_mock_project_model(
        id: Uuid,
        user_id: Uuid,
        title: &str,
        slug: &str,
    ) -> projects::Model {
        let now = Utc::now().fixed_offset();

        projects::Model {
            id,
            user_id,
            title: title.to_string(),
            slug: slug.to_string(),
            description: "Test description".to_string(),
            tech_stack: vec!["Rust".to_string()],
            screenshots: serde_json::json!(["img1.png"]),
            repo_url: Some("https://github.com/test/repo".to_string()),
            live_demo_url: Some("https://demo.test.com".to_string()),
            is_deleted: false,
            created_at: now,
            updated_at: now,
        }
    }

    // ========================================================================
    // create_project Tests
    // ========================================================================

    #[tokio::test]
    async fn test_create_project_success() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let mut data = create_test_project_data();
        data.owner = UserId::from(user_id);

        let mock_model =
            create_mock_project_model(project_id, user_id, "Test Project", "test-project");

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_model]])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo.create_project(data).await;

        assert!(result.is_ok());
        let project = result.unwrap();
        assert_eq!(project.title, "Test Project");
        assert_eq!(project.slug, "test-project");
    }

    #[tokio::test]
    async fn test_create_project_trims_title() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let mut data = create_test_project_data();
        data.owner = UserId::from(user_id);
        data.title = "  Trimmed Title  ".to_string();

        let mock_model =
            create_mock_project_model(project_id, user_id, "Trimmed Title", "test-project");

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_model]])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo.create_project(data).await;

        assert!(result.is_ok());
        let project = result.unwrap();
        assert_eq!(project.title, "Trimmed Title");
    }

    #[tokio::test]
    async fn test_create_project_lowercase_slug() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let mut data = create_test_project_data();
        data.owner = UserId::from(user_id);
        data.slug = "UPPERCASE-SLUG".to_string();

        let mock_model =
            create_mock_project_model(project_id, user_id, "Test Project", "uppercase-slug");

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_model]])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo.create_project(data).await;

        assert!(result.is_ok());
        let project = result.unwrap();
        assert_eq!(project.slug, "uppercase-slug");
    }

    #[tokio::test]
    async fn test_create_project_with_null_urls() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let mut data = create_test_project_data();
        data.owner = UserId::from(user_id);
        data.repo_url = None;
        data.live_demo_url = None;

        let mut mock_model =
            create_mock_project_model(project_id, user_id, "Test Project", "test-project");
        mock_model.repo_url = None;
        mock_model.live_demo_url = None;

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_model]])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo.create_project(data).await;

        assert!(result.is_ok());
        let project = result.unwrap();
        assert!(project.repo_url.is_none());
        assert!(project.live_demo_url.is_none());
    }

    #[tokio::test]
    async fn test_create_project_slug_already_exists() {
        let data = create_test_project_data();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors(vec![DbErr::Custom(
                "duplicate key value violates unique constraint \"idx_projects_slug_unique\""
                    .to_string(),
            )])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo.create_project(data).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ProjectRepositoryError::SlugAlreadyExists
        ));
    }

    #[tokio::test]
    async fn test_create_project_database_error() {
        let data = create_test_project_data();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors(vec![DbErr::Custom("connection timeout".to_string())])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo.create_project(data).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ProjectRepositoryError::DatabaseError(msg) => {
                assert!(msg.contains("connection timeout"));
            }
            _ => panic!("Expected DatabaseError"),
        }
    }

    // ========================================================================
    // patch_project Tests
    // ========================================================================

    #[tokio::test]
    async fn test_patch_project_update_title() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mock_model =
            create_mock_project_model(project_id, user_id, "Updated Title", "test-slug");

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_model]])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo
            .patch_project(
                UserId::from(user_id),
                project_id,
                PatchProjectData {
                    title: PatchField::Value("Updated Title".to_string()),
                    ..Default::default()
                },
            )
            .await;

        assert!(result.is_ok());
        let project = result.unwrap();
        assert_eq!(project.title, "Updated Title");
    }

    #[tokio::test]
    async fn test_patch_project_update_description() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mut mock_model = create_mock_project_model(project_id, user_id, "Title", "test-slug");
        mock_model.description = "New Description".to_string();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_model]])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo
            .patch_project(
                UserId::from(user_id),
                project_id,
                PatchProjectData {
                    description: PatchField::Value("New Description".to_string()),
                    ..Default::default()
                },
            )
            .await;

        assert!(result.is_ok());
        let project = result.unwrap();
        assert_eq!(project.description, "New Description");
    }

    #[tokio::test]
    async fn test_patch_project_update_tech_stack() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let new_tech = vec!["Python".to_string(), "Django".to_string()];
        let mut mock_model = create_mock_project_model(project_id, user_id, "Title", "test-slug");
        mock_model.tech_stack = new_tech.clone();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_model]])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo
            .patch_project(
                UserId::from(user_id),
                project_id,
                PatchProjectData {
                    tech_stack: PatchField::Value(new_tech.clone()),
                    ..Default::default()
                },
            )
            .await;

        assert!(result.is_ok());
        let project = result.unwrap();
        assert_eq!(project.tech_stack, new_tech);
    }

    #[tokio::test]
    async fn test_patch_project_update_screenshots() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let new_screenshots = vec!["new1.png".to_string(), "new2.png".to_string()];
        let mut mock_model = create_mock_project_model(project_id, user_id, "Title", "test-slug");
        mock_model.screenshots = serde_json::to_value(&new_screenshots).unwrap();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_model]])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo
            .patch_project(
                UserId::from(user_id),
                project_id,
                PatchProjectData {
                    screenshots: PatchField::Value(new_screenshots.clone()),
                    ..Default::default()
                },
            )
            .await;

        assert!(result.is_ok());
        let project = result.unwrap();
        assert_eq!(project.screenshots, new_screenshots);
    }

    #[tokio::test]
    async fn test_patch_project_set_repo_url_to_null() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mut mock_model = create_mock_project_model(project_id, user_id, "Title", "test-slug");
        mock_model.repo_url = None;

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_model]])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo
            .patch_project(
                UserId::from(user_id),
                project_id,
                PatchProjectData {
                    repo_url: PatchField::Null,
                    ..Default::default()
                },
            )
            .await;

        assert!(result.is_ok());
        let project = result.unwrap();
        assert!(project.repo_url.is_none());
    }

    #[tokio::test]
    async fn test_patch_project_set_repo_url_to_value() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let new_url = "https://github.com/new/repo".to_string();
        let mut mock_model = create_mock_project_model(project_id, user_id, "Title", "test-slug");
        mock_model.repo_url = Some(new_url.clone());

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_model]])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo
            .patch_project(
                UserId::from(user_id),
                project_id,
                PatchProjectData {
                    repo_url: PatchField::Value(new_url.clone()),
                    ..Default::default()
                },
            )
            .await;

        assert!(result.is_ok());
        let project = result.unwrap();
        assert_eq!(project.repo_url, Some(new_url));
    }

    #[tokio::test]
    async fn test_patch_project_set_live_demo_url_to_null() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mut mock_model = create_mock_project_model(project_id, user_id, "Title", "test-slug");
        mock_model.live_demo_url = None;

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_model]])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo
            .patch_project(
                UserId::from(user_id),
                project_id,
                PatchProjectData {
                    live_demo_url: PatchField::Null,
                    ..Default::default()
                },
            )
            .await;

        assert!(result.is_ok());
        let project = result.unwrap();
        assert!(project.live_demo_url.is_none());
    }

    #[tokio::test]
    async fn test_patch_project_set_live_demo_url_to_value() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let new_url = "https://newdemo.example.com".to_string();
        let mut mock_model = create_mock_project_model(project_id, user_id, "Title", "test-slug");
        mock_model.live_demo_url = Some(new_url.clone());

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_model]])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo
            .patch_project(
                UserId::from(user_id),
                project_id,
                PatchProjectData {
                    live_demo_url: PatchField::Value(new_url.clone()),
                    ..Default::default()
                },
            )
            .await;

        assert!(result.is_ok());
        let project = result.unwrap();
        assert_eq!(project.live_demo_url, Some(new_url));
    }

    #[tokio::test]
    async fn test_patch_project_multiple_fields() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let new_tech = vec!["Go".to_string()];
        let mut mock_model =
            create_mock_project_model(project_id, user_id, "New Title", "test-slug");
        mock_model.description = "New Desc".to_string();
        mock_model.tech_stack = new_tech.clone();
        mock_model.repo_url = None;

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_model]])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo
            .patch_project(
                UserId::from(user_id),
                project_id,
                PatchProjectData {
                    title: PatchField::Value("New Title".to_string()),
                    description: PatchField::Value("New Desc".to_string()),
                    tech_stack: PatchField::Value(new_tech.clone()),
                    repo_url: PatchField::Null,
                    ..Default::default()
                },
            )
            .await;

        assert!(result.is_ok());
        let project = result.unwrap();
        assert_eq!(project.title, "New Title");
        assert_eq!(project.description, "New Desc");
        assert_eq!(project.tech_stack, new_tech);
        assert!(project.repo_url.is_none());
    }

    #[tokio::test]
    async fn test_patch_project_no_changes_returns_current_state() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mock_model =
            create_mock_project_model(project_id, user_id, "Original Title", "test-slug");

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_model]])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo
            .patch_project(
                UserId::from(user_id),
                project_id,
                PatchProjectData::default(),
            )
            .await;

        assert!(result.is_ok());
        let project = result.unwrap();
        assert_eq!(project.title, "Original Title");
    }

    #[tokio::test]
    async fn test_patch_project_not_found() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<projects::Model>::new()])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo
            .patch_project(
                UserId::from(user_id),
                project_id,
                PatchProjectData {
                    title: PatchField::Value("New Title".to_string()),
                    ..Default::default()
                },
            )
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ProjectRepositoryError::NotFound
        ));
    }

    #[tokio::test]
    async fn test_patch_project_unauthorized_user() {
        let project_id = Uuid::new_v4();
        let wrong_user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<projects::Model>::new()])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo
            .patch_project(
                UserId::from(wrong_user_id),
                project_id,
                PatchProjectData {
                    title: PatchField::Value("New Title".to_string()),
                    ..Default::default()
                },
            )
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ProjectRepositoryError::NotFound
        ));
    }

    #[tokio::test]
    async fn test_patch_project_database_error() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors(vec![DbErr::Custom("database connection lost".to_string())])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo
            .patch_project(
                UserId::from(user_id),
                project_id,
                PatchProjectData {
                    title: PatchField::Value("New Title".to_string()),
                    ..Default::default()
                },
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ProjectRepositoryError::DatabaseError(msg) => {
                assert!(msg.contains("database connection lost"));
            }
            _ => panic!("Expected DatabaseError"),
        }
    }

    #[tokio::test]
    async fn test_patch_project_trims_title() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mock_model = create_mock_project_model(project_id, user_id, "Trimmed", "test-slug");

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_model]])
            .into_connection();

        let repo = ProjectRepositoryPostgres::new(Arc::new(db));
        let result = repo
            .patch_project(
                UserId::from(user_id),
                project_id,
                PatchProjectData {
                    title: PatchField::Value("  Trimmed  ".to_string()),
                    ..Default::default()
                },
            )
            .await;

        assert!(result.is_ok());
        let project = result.unwrap();
        assert_eq!(project.title, "Trimmed");
    }

    // ========================================================================
    // Helper Function Tests
    // ========================================================================

    #[test]
    fn test_to_json_success() {
        let data = vec!["test".to_string()];
        let result = to_json(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_from_json_success() {
        let json = serde_json::json!(["test"]);
        let result: Result<Vec<String>, _> = from_json(&json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["test"]);
    }

    #[test]
    fn test_from_json_error() {
        let json = serde_json::json!("not an array");
        let result: Result<Vec<String>, _> = from_json(&json);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ProjectRepositoryError::SerializationError(_)
        ));
    }

    #[test]
    fn test_map_slug_error_duplicate() {
        let err = DbErr::Custom(
            "duplicate key value violates unique constraint \"idx_projects_slug_unique\""
                .to_string(),
        );
        let result = map_slug_error(err);
        assert!(matches!(result, ProjectRepositoryError::SlugAlreadyExists));
    }

    #[test]
    fn test_map_slug_error_other() {
        let err = DbErr::Custom("some other error".to_string());
        let result = map_slug_error(err);
        assert!(matches!(result, ProjectRepositoryError::DatabaseError(_)));
    }
}
