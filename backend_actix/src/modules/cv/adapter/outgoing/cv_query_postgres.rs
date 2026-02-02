use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::prelude::Expr;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::QuerySelect;
use sea_orm::{ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::cv::application::ports::outgoing::{
    CVListFilter, CVPageRequest, CVPageResult, CVQuery, CVQueryError, CVSort,
};
use crate::cv::domain::entities::CVInfo;

// Adjust these to your actual generated entity path
use crate::modules::cv::adapter::outgoing::sea_orm_entity::{
    Column as ResumeColumn, Entity as ResumeEntity, Model as ResumeModel,
};

#[derive(Debug, Clone)]
pub struct CVQueryPostgres {
    db: Arc<sea_orm::DatabaseConnection>,
}

impl CVQueryPostgres {
    pub fn new(db: Arc<sea_orm::DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl CVQuery for CVQueryPostgres {
    async fn list(
        &self,
        user_id: Uuid,
        filter: CVListFilter,
        sort: CVSort,
        page: CVPageRequest,
    ) -> Result<CVPageResult<CVInfo>, CVQueryError> {
        // Base query (active resumes only)
        let mut query = ResumeEntity::find()
            .filter(ResumeColumn::UserId.eq(user_id))
            .filter(ResumeColumn::IsDeleted.eq(false));

        // Optional search: display_name + role (both exist in schema)
        if let Some(ref search) = filter.search {
            let term = search.trim();
            if !term.is_empty() {
                let pattern = format!("%{}%", term);

                // For JSONB array-of-objects fields, cast the whole column to text
                // and do ILIKE. This searches all nested string values.
                let core_skills_expr =
                    Expr::cust_with_values("CAST(core_skills AS TEXT) ILIKE $1", [pattern.clone()]);

                let educations_expr =
                    Expr::cust_with_values("CAST(educations AS TEXT) ILIKE $1", [pattern.clone()]);

                let experiences_expr =
                    Expr::cust_with_values("CAST(experiences AS TEXT) ILIKE $1", [pattern.clone()]);

                let contact_info_expr = Expr::cust_with_values(
                    "CAST(contact_info AS TEXT) ILIKE $1",
                    [pattern.clone()],
                );
                query = query.filter(
                    Condition::any()
                        .add(Expr::col(ResumeColumn::DisplayName).ilike(&pattern))
                        .add(Expr::col(ResumeColumn::Role).ilike(&pattern))
                        .add(core_skills_expr)
                        .add(educations_expr)
                        .add(experiences_expr)
                        .add(contact_info_expr),
                );
            }
        }

        // Sorting
        query = match sort {
            CVSort::Newest => query.order_by_desc(ResumeColumn::CreatedAt),
            CVSort::Oldest => query.order_by_asc(ResumeColumn::CreatedAt),
            CVSort::UpdatedNewest => query.order_by_desc(ResumeColumn::UpdatedAt),
            CVSort::UpdatedOldest => query.order_by_asc(ResumeColumn::UpdatedAt),
        };

        // total count
        let total = query
            .clone()
            .count(&*self.db)
            .await
            .map_err(|e| CVQueryError::DatabaseError(e.to_string()))?;

        // pagination
        let offset = ((page.page.saturating_sub(1)) * page.per_page) as u64;

        let models: Vec<ResumeModel> = query
            .offset(offset)
            .limit(page.per_page as u64)
            .all(&*self.db)
            .await
            .map_err(|e| CVQueryError::DatabaseError(e.to_string()))?;

        Ok(CVPageResult {
            items: models.into_iter().map(|m| m.to_domain()).collect(),
            page: page.page,
            per_page: page.per_page,
            total,
        })
    }

    async fn fetch_cv_by_id(&self, cv_id: Uuid) -> Result<Option<CVInfo>, CVQueryError> {
        let model: Option<ResumeModel> = ResumeEntity::find_by_id(cv_id)
            .filter(ResumeColumn::IsDeleted.eq(false))
            .one(&*self.db)
            .await
            .map_err(|err| CVQueryError::DatabaseError(err.to_string()))?;

        Ok(model.map(|m| m.to_domain()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase};

    fn create_mock_resume_model(
        id: Uuid,
        user_id: Uuid,
        display_name: &str,
        role: &str,
    ) -> ResumeModel {
        let now = Utc::now().fixed_offset();

        ResumeModel {
            id,
            user_id,
            display_name: display_name.to_string(),
            role: role.to_string(),
            bio: "Test bio".to_string(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            core_skills: serde_json::json!([{"name": "Rust", "level": "advanced"}]),
            educations: serde_json::json!([{"school": "MIT", "degree": "CS"}]),
            experiences: serde_json::json!([{"company": "Acme", "role": "Engineer"}]),
            highlighted_projects: serde_json::json!([{"title": "Portfolio"}]),
            contact_info: serde_json::json!([{"type": "email", "value": "test@test.com"}]),
            created_at: now,
            updated_at: now,
            is_deleted: false,
        }
    }

    // ========================================================================
    // list Tests
    // ========================================================================

    #[tokio::test]
    async fn test_list_database_error() {
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors(vec![sea_orm::DbErr::Custom("connection error".to_string())])
            .into_connection();

        let query = CVQueryPostgres::new(Arc::new(db));
        let result = query
            .list(
                user_id,
                CVListFilter::default(),
                CVSort::default(),
                CVPageRequest::default(),
            )
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CVQueryError::DatabaseError(_)
        ));
    }

    // Note: list() uses count() which is difficult to mock with MockDatabase.
    // Use integration tests for full list coverage including:
    // - successful listing with results
    // - empty results
    // - search filter (empty string, whitespace, with matches)
    // - all sort variants (Newest, Oldest, UpdatedNewest, UpdatedOldest)
    // - pagination edge cases (page 0, page beyond total)

    // ========================================================================
    // fetch_cv_by_id Tests
    // ========================================================================

    #[tokio::test]
    async fn test_fetch_cv_by_id_success() {
        let cv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mock_resume = create_mock_resume_model(cv_id, user_id, "John Doe", "Backend Engineer");

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_resume]])
            .into_connection();

        let query = CVQueryPostgres::new(Arc::new(db));
        let result = query.fetch_cv_by_id(cv_id).await;

        assert!(result.is_ok());
        let cv = result.unwrap();
        assert!(cv.is_some());
        let cv = cv.unwrap();
        assert_eq!(cv.id, cv_id);
        assert_eq!(cv.display_name, "John Doe");
        assert_eq!(cv.role, "Backend Engineer");
    }

    #[tokio::test]
    async fn test_fetch_cv_by_id_not_found() {
        let cv_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<ResumeModel>::new()])
            .into_connection();

        let query = CVQueryPostgres::new(Arc::new(db));
        let result = query.fetch_cv_by_id(cv_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_fetch_cv_by_id_database_error() {
        let cv_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors(vec![sea_orm::DbErr::Custom("connection error".to_string())])
            .into_connection();

        let query = CVQueryPostgres::new(Arc::new(db));
        let result = query.fetch_cv_by_id(cv_id).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CVQueryError::DatabaseError(_)
        ));
    }
}
