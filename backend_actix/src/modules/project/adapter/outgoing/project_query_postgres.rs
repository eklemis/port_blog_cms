// src/modules/project/adapter/outgoing/project_query_postgres.rs

use async_trait::async_trait;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::{
    sea_query::Expr, ColumnTrait, Condition, DatabaseConnection, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::adapter::outgoing::sea_orm_entity::project_topics;
use crate::modules::project::adapter::outgoing::sea_orm_entity::projects::{self, Column, Entity};
use crate::modules::project::application::ports::outgoing::project_query::{
    PageRequest, PageResult, ProjectCardView, ProjectListFilter, ProjectQuery, ProjectQueryError,
    ProjectSort, ProjectView,
};

// ============================================================================
// Repository Implementation
// ============================================================================

#[derive(Clone)]
pub struct ProjectQueryPostgres {
    db: Arc<DatabaseConnection>,
}

impl ProjectQueryPostgres {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ProjectQuery for ProjectQueryPostgres {
    async fn get_by_id(
        &self,
        owner: UserId,
        project_id: Uuid,
    ) -> Result<ProjectView, ProjectQueryError> {
        let owner_uuid: Uuid = owner.into();

        let project = Entity::find_by_id(project_id)
            .filter(Column::UserId.eq(owner_uuid))
            .filter(Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await
            .map_err(map_db_err)?
            .ok_or(ProjectQueryError::NotFound)?;

        let topic_ids = self.get_project_topics(project_id).await?;

        model_to_view(project, topic_ids)
    }

    async fn get_by_slug(&self, slug: &str) -> Result<ProjectView, ProjectQueryError> {
        let normalized_slug = slug.trim().to_lowercase();

        let project = Entity::find()
            .filter(Column::Slug.eq(&normalized_slug))
            .filter(Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await
            .map_err(map_db_err)?
            .ok_or(ProjectQueryError::NotFound)?;

        let topic_ids = self.get_project_topics(project.id).await?;

        model_to_view(project, topic_ids)
    }

    async fn list(
        &self,
        owner: UserId,
        filter: ProjectListFilter,
        sort: ProjectSort,
        page: PageRequest,
    ) -> Result<PageResult<ProjectCardView>, ProjectQueryError> {
        let owner_uuid: Uuid = owner.into();

        // Base query
        let mut query = Entity::find()
            .filter(Column::UserId.eq(owner_uuid))
            .filter(Column::IsDeleted.eq(false));

        // Apply search filter with ILIKE
        if let Some(ref search) = filter.search {
            let search_pattern = format!("%{}%", search.trim());
            query = query.filter(
                Condition::any()
                    .add(Expr::col(Column::Title).ilike(&search_pattern))
                    .add(Expr::col(Column::Description).ilike(&search_pattern)),
            );
        }

        // Apply topic filter via subquery
        if let Some(topic_id) = filter.topic_id {
            let project_ids_with_topic = project_topics::Entity::find()
                .filter(project_topics::Column::TopicId.eq(topic_id))
                .select_only()
                .column(project_topics::Column::ProjectId)
                .into_tuple::<Uuid>()
                .all(&*self.db)
                .await
                .map_err(map_db_err)?;

            query = query.filter(Column::Id.is_in(project_ids_with_topic));
        }

        // Apply sorting
        query = match sort {
            ProjectSort::Newest => query.order_by_desc(Column::CreatedAt),
            ProjectSort::Oldest => query.order_by_asc(Column::CreatedAt),
            ProjectSort::UpdatedNewest => query.order_by_desc(Column::UpdatedAt),
            ProjectSort::UpdatedOldest => query.order_by_asc(Column::UpdatedAt),
        };

        // Get total count
        let total = query.clone().count(&*self.db).await.map_err(map_db_err)?;

        // Apply pagination
        let offset = ((page.page.saturating_sub(1)) * page.per_page) as u64;
        let projects = query
            .offset(offset)
            .limit(page.per_page as u64)
            .all(&*self.db)
            .await
            .map_err(map_db_err)?;

        // Map to card views
        let items: Result<Vec<ProjectCardView>, ProjectQueryError> =
            projects.into_iter().map(model_to_card_view).collect();

        Ok(PageResult {
            items: items?,
            page: page.page,
            per_page: page.per_page,
            total,
        })
    }

    async fn get_project_topics(&self, project_id: Uuid) -> Result<Vec<Uuid>, ProjectQueryError> {
        let topic_ids = project_topics::Entity::find()
            .filter(project_topics::Column::ProjectId.eq(project_id))
            .select_only()
            .column(project_topics::Column::TopicId)
            .into_tuple::<Uuid>()
            .all(&*self.db)
            .await
            .map_err(map_db_err)?;

        Ok(topic_ids)
    }

    async fn slug_exists(&self, slug: &str) -> Result<bool, ProjectQueryError> {
        let normalized_slug = slug.trim().to_lowercase();

        let count = Entity::find()
            .filter(Column::Slug.eq(&normalized_slug))
            .filter(Column::IsDeleted.eq(false))
            .count(&*self.db)
            .await
            .map_err(map_db_err)?;

        Ok(count > 0)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn model_to_view(
    model: projects::Model,
    topic_ids: Vec<Uuid>,
) -> Result<ProjectView, ProjectQueryError> {
    Ok(ProjectView {
        id: model.id,
        owner: UserId::from(model.user_id),
        title: model.title,
        slug: model.slug,
        description: model.description,
        tech_stack: from_json(&model.tech_stack)?,
        screenshots: from_json(&model.screenshots)?,
        repo_url: model.repo_url,
        live_demo_url: model.live_demo_url,
        topic_ids,
        created_at: model.created_at.into(),
        updated_at: model.updated_at.into(),
    })
}

fn model_to_card_view(model: projects::Model) -> Result<ProjectCardView, ProjectQueryError> {
    Ok(ProjectCardView {
        id: model.id,
        title: model.title,
        slug: model.slug,
        tech_stack: from_json(&model.tech_stack)?,
        repo_url: model.repo_url,
        live_demo_url: model.live_demo_url,
        created_at: model.created_at.into(),
        updated_at: model.updated_at.into(),
    })
}

fn from_json<T: serde::de::DeserializeOwned>(
    json: &serde_json::Value,
) -> Result<T, ProjectQueryError> {
    serde_json::from_value(json.clone())
        .map_err(|e| ProjectQueryError::SerializationError(e.to_string()))
}

fn map_db_err(e: DbErr) -> ProjectQueryError {
    ProjectQueryError::DatabaseError(e.to_string())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::sea_query::Value;
    use sea_orm::{DatabaseBackend, MockDatabase};
    use std::collections::BTreeMap;

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
            tech_stack: serde_json::json!(["Rust".to_string()]),
            screenshots: serde_json::json!(["img1.png"]),
            repo_url: Some("https://github.com/test/repo".to_string()),
            live_demo_url: Some("https://demo.test.com".to_string()),
            is_deleted: false,
            created_at: now,
            updated_at: now,
        }
    }

    // ========================================================================
    // get_by_id Tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_by_id_success() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let topic_id = Uuid::new_v4();

        let mock_project =
            create_mock_project_model(project_id, user_id, "Test Project", "test-project");

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_project]]) // 1st query: projects::Model
            .append_query_results(vec![vec![BTreeMap::from([(
                "topic_id".to_string(),
                Value::Uuid(Some(Box::new(topic_id))),
            )])]]) // 2nd query: projection => one-column row
            .into_connection();

        let query = ProjectQueryPostgres::new(Arc::new(db));
        let result = query.get_by_id(UserId::from(user_id), project_id).await;

        assert!(result.is_ok());
        let view = result.unwrap();
        assert_eq!(view.id, project_id);
        assert_eq!(view.title, "Test Project");
        assert_eq!(view.topic_ids.len(), 1);
        assert_eq!(view.topic_ids[0], topic_id);
    }

    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<projects::Model>::new()])
            .into_connection();

        let query = ProjectQueryPostgres::new(Arc::new(db));
        let result = query.get_by_id(UserId::from(user_id), project_id).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ProjectQueryError::NotFound));
    }

    #[tokio::test]
    async fn test_get_by_id_no_topics() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mock_project =
            create_mock_project_model(project_id, user_id, "Test Project", "test-project");

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_project]])
            .append_query_results(vec![Vec::<project_topics::Model>::new()])
            .into_connection();

        let query = ProjectQueryPostgres::new(Arc::new(db));
        let result = query.get_by_id(UserId::from(user_id), project_id).await;

        assert!(result.is_ok());
        let view = result.unwrap();
        assert!(view.topic_ids.is_empty());
    }

    // ========================================================================
    // get_by_slug Tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_by_slug_success() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mock_project =
            create_mock_project_model(project_id, user_id, "Test Project", "test-project");

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_project]])
            .append_query_results(vec![Vec::<project_topics::Model>::new()])
            .into_connection();

        let query = ProjectQueryPostgres::new(Arc::new(db));
        let result = query.get_by_slug("test-project").await;

        assert!(result.is_ok());
        let view = result.unwrap();
        assert_eq!(view.slug, "test-project");
    }

    #[tokio::test]
    async fn test_get_by_slug_normalizes_input() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mock_project =
            create_mock_project_model(project_id, user_id, "Test Project", "test-project");

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_project]])
            .append_query_results(vec![Vec::<project_topics::Model>::new()])
            .into_connection();

        let query = ProjectQueryPostgres::new(Arc::new(db));
        let result = query.get_by_slug("  TEST-PROJECT  ").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_by_slug_not_found() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<projects::Model>::new()])
            .into_connection();

        let query = ProjectQueryPostgres::new(Arc::new(db));
        let result = query.get_by_slug("nonexistent").await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ProjectQueryError::NotFound));
    }

    // ========================================================================
    // get_project_topics Tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_project_topics_success() {
        use sea_orm::sea_query::Value;
        use std::collections::BTreeMap;

        let project_id = Uuid::new_v4();
        let topic_id_1 = Uuid::new_v4();
        let topic_id_2 = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![
                BTreeMap::from([(
                    "topic_id".to_string(),
                    Value::Uuid(Some(Box::new(topic_id_1))),
                )]),
                BTreeMap::from([(
                    "topic_id".to_string(),
                    Value::Uuid(Some(Box::new(topic_id_2))),
                )]),
            ]])
            .into_connection();

        let query = ProjectQueryPostgres::new(Arc::new(db));
        let result = query.get_project_topics(project_id).await;

        assert!(result.is_ok());
        let topics = result.unwrap();
        assert_eq!(topics.len(), 2);
        assert!(topics.contains(&topic_id_1));
        assert!(topics.contains(&topic_id_2));
    }

    #[tokio::test]
    async fn test_get_project_topics_empty() {
        let project_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<project_topics::Model>::new()])
            .into_connection();

        let query = ProjectQueryPostgres::new(Arc::new(db));
        let result = query.get_project_topics(project_id).await;

        assert!(result.is_ok());
        let topics = result.unwrap();
        assert!(topics.is_empty());
    }

    // ========================================================================
    // slug_exists Tests
    // ========================================================================

    #[tokio::test]
    async fn test_slug_exists_database_error() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors(vec![sea_orm::DbErr::Custom("connection error".to_string())])
            .into_connection();

        let query = ProjectQueryPostgres::new(Arc::new(db));
        let result = query.slug_exists("any-slug").await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ProjectQueryError::DatabaseError(_)
        ));
    }

    // Note: count() is difficult to mock with MockDatabase.
    // Use integration tests for full slug_exists coverage.

    // ========================================================================
    // list Tests - Basic coverage
    // ========================================================================

    #[tokio::test]
    async fn test_list_database_error() {
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors(vec![sea_orm::DbErr::Custom("connection error".to_string())])
            .into_connection();

        let query = ProjectQueryPostgres::new(Arc::new(db));
        let result = query
            .list(
                UserId::from(user_id),
                ProjectListFilter::default(),
                ProjectSort::default(),
                PageRequest::default(),
            )
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ProjectQueryError::DatabaseError(_)
        ));
    }

    // Note: list() uses count() which is difficult to mock.
    // Use integration tests for full list coverage.

    // ========================================================================
    // Helper Function Tests
    // ========================================================================

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
            ProjectQueryError::SerializationError(_)
        ));
    }

    #[test]
    fn test_model_to_card_view() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let model = create_mock_project_model(project_id, user_id, "Test", "test-slug");

        let result = model_to_card_view(model);

        assert!(result.is_ok());
        let card = result.unwrap();
        assert_eq!(card.id, project_id);
        assert_eq!(card.title, "Test");
        assert_eq!(card.slug, "test-slug");
    }

    #[test]
    fn test_model_to_view() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let topic_id = Uuid::new_v4();
        let model = create_mock_project_model(project_id, user_id, "Test", "test-slug");

        let result = model_to_view(model, vec![topic_id]);

        assert!(result.is_ok());
        let view = result.unwrap();
        assert_eq!(view.id, project_id);
        assert_eq!(view.topic_ids.len(), 1);
        assert_eq!(view.topic_ids[0], topic_id);
    }
}
