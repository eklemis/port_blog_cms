use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, RuntimeErr,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::blog::adapter::outgoing::sea_orm_entity::blog_posts::{
    ActiveModel as PostActive, Column as PostColumn, Entity as PostEntity,
};
use crate::blog::application::ports::outgoing::{
    BlogPatchField, BlogPostRepository, BlogPostRepositoryError, CreateBlogPostData,
    PatchBlogPostData,
};
use crate::blog::domain::entities::BlogPost;

#[derive(Clone)]
pub struct BlogPostRepositoryPostgres {
    db: Arc<DatabaseConnection>,
}

impl BlogPostRepositoryPostgres {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Maps the per-author slug unique index onto a domain error.
    ///
    /// Postgres reports a unique violation as SQLSTATE 23505; matching on the
    /// index name keeps a collision on some future index from being
    /// misreported as a slug clash.
    fn map_err(e: DbErr) -> BlogPostRepositoryError {
        let text = match &e {
            DbErr::Query(RuntimeErr::SqlxError(inner)) => inner.to_string(),
            other => other.to_string(),
        };

        if text.contains("idx_blog_posts_user_slug_unique") {
            BlogPostRepositoryError::SlugAlreadyExists
        } else {
            BlogPostRepositoryError::DatabaseError(e.to_string())
        }
    }
}

#[async_trait]
impl BlogPostRepository for BlogPostRepositoryPostgres {
    async fn create(&self, data: CreateBlogPostData) -> Result<BlogPost, BlogPostRepositoryError> {
        let model = PostActive {
            id: Set(Uuid::new_v4()),
            user_id: Set(data.owner.value()),
            title: Set(data.title.trim().to_string()),
            slug: Set(data.slug.trim().to_lowercase()),
            excerpt: Set(data.excerpt.map(|e| e.trim().to_string())),
            content: Set(data.content),
            published_at: Set(data.published_at.map(|t| t.into())),
            is_deleted: Set(false),
            ..Default::default()
        };

        let inserted = model.insert(&*self.db).await.map_err(Self::map_err)?;
        Ok(inserted.to_domain())
    }

    async fn fetch_by_id(
        &self,
        post_id: Uuid,
    ) -> Result<Option<BlogPost>, BlogPostRepositoryError> {
        // Intentionally unfiltered: callers need soft-deleted and unpublished
        // posts visible to run ownership checks before acting on them.
        let found = PostEntity::find_by_id(post_id)
            .one(&*self.db)
            .await
            .map_err(Self::map_err)?;

        Ok(found.map(|m| m.to_domain()))
    }

    async fn patch(
        &self,
        post_id: Uuid,
        data: PatchBlogPostData,
    ) -> Result<BlogPost, BlogPostRepositoryError> {
        let existing = PostEntity::find_by_id(post_id)
            .filter(PostColumn::IsDeleted.eq(false))
            .one(&*self.db)
            .await
            .map_err(Self::map_err)?
            .ok_or(BlogPostRepositoryError::NotFound)?;

        let mut active: PostActive = existing.into();

        // Unset leaves the column alone; Null and Value both write. That is the
        // whole reason for BlogPatchField — unpublishing is a write of NULL,
        // not an absence of input.
        if let BlogPatchField::Value(v) = &data.title {
            active.title = Set(v.trim().to_string());
        }
        if let BlogPatchField::Value(v) = &data.slug {
            active.slug = Set(v.trim().to_lowercase());
        }
        if let BlogPatchField::Value(v) = &data.content {
            active.content = Set(v.clone());
        }

        match &data.excerpt {
            BlogPatchField::Unset => {}
            BlogPatchField::Null => active.excerpt = Set(None),
            BlogPatchField::Value(v) => active.excerpt = Set(Some(v.trim().to_string())),
        }

        match &data.published_at {
            BlogPatchField::Unset => {}
            BlogPatchField::Null => active.published_at = Set(None),
            BlogPatchField::Value(t) => active.published_at = Set(Some((*t).into())),
        }

        let updated = active.update(&*self.db).await.map_err(Self::map_err)?;
        Ok(updated.to_domain())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::domain::entities::UserId;
    use crate::blog::adapter::outgoing::sea_orm_entity::blog_posts::Model as PostModel;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase};

    fn model(user_id: Uuid, published_at: Option<chrono::DateTime<Utc>>) -> PostModel {
        let now = Utc::now().fixed_offset();
        PostModel {
            id: Uuid::new_v4(),
            user_id,
            title: "Hello".into(),
            slug: "hello".into(),
            excerpt: Some("summary".into()),
            content: "body".into(),
            published_at: published_at.map(|t| t.fixed_offset()),
            is_deleted: false,
            created_at: now,
            updated_at: now,
        }
    }

    fn repo(db: DatabaseConnection) -> BlogPostRepositoryPostgres {
        BlogPostRepositoryPostgres::new(Arc::new(db))
    }

    fn create_data(owner: UserId) -> CreateBlogPostData {
        CreateBlogPostData {
            owner,
            title: "  Hello  ".into(),
            slug: "  HeLLo  ".into(),
            excerpt: Some("  summary  ".into()),
            content: "body".into(),
            published_at: None,
        }
    }

    #[tokio::test]
    async fn create_returns_the_inserted_post() {
        let user_id = Uuid::new_v4();
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![model(user_id, None)]])
            .into_connection();

        let post = repo(db)
            .create(create_data(UserId::from(user_id)))
            .await
            .unwrap();

        assert_eq!(post.user_id, user_id);
        assert_eq!(post.published_at, None);
    }

    /// The unique index is on `lower(slug)`, so slugs are normalised on write.
    /// Otherwise "Hello" and "hello" would collide at the database with an
    /// error the caller could not have predicted from what it sent.
    #[tokio::test]
    async fn create_trims_and_lowercases_the_slug() {
        let user_id = Uuid::new_v4();
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![model(user_id, None)]])
            .into_connection();

        // The mock echoes a fixed row, so assert on what was sent rather than
        // what came back: the statement log is the source of truth here. The
        // repo is dropped first so the Arc has a single owner to unwrap.
        let conn = Arc::new(db);
        let repo = BlogPostRepositoryPostgres::new(Arc::clone(&conn));
        repo.create(create_data(UserId::from(user_id)))
            .await
            .unwrap();
        drop(repo);

        let log = Arc::try_unwrap(conn)
            .expect("repo dropped, so the Arc is sole-owned")
            .into_transaction_log();
        let sql = format!("{log:?}").replace("\\\"", "\"");
        assert!(sql.contains("hello"), "slug should be lowercased: {sql}");
        assert!(
            !sql.contains("HeLLo"),
            "raw slug should not reach the database: {sql}"
        );
    }

    /// A slug collision surfaces as SQLSTATE 23505 naming the index. Matching
    /// the index name rather than just "unique violation" keeps a future
    /// constraint from being misreported as a slug clash.
    #[tokio::test]
    async fn create_maps_the_slug_index_violation_to_a_domain_error() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors([DbErr::Custom(
                "duplicate key value violates unique constraint \
                 \"idx_blog_posts_user_slug_unique\""
                    .to_string(),
            )])
            .into_connection();

        let err = repo(db)
            .create(create_data(UserId::from(Uuid::new_v4())))
            .await
            .unwrap_err();

        assert!(matches!(err, BlogPostRepositoryError::SlugAlreadyExists));
    }

    #[tokio::test]
    async fn create_surfaces_other_database_errors_unchanged() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors([DbErr::Custom("connection reset".to_string())])
            .into_connection();

        let err = repo(db)
            .create(create_data(UserId::from(Uuid::new_v4())))
            .await
            .unwrap_err();

        assert!(
            matches!(err, BlogPostRepositoryError::DatabaseError(m) if m.contains("connection reset"))
        );
    }

    #[tokio::test]
    async fn fetch_by_id_returns_none_when_absent() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<PostModel>::new()])
            .into_connection();

        assert!(repo(db)
            .fetch_by_id(Uuid::new_v4())
            .await
            .unwrap()
            .is_none());
    }

    /// Unfiltered by design: services need soft-deleted and unpublished posts
    /// visible so they can run ownership checks before acting.
    #[tokio::test]
    async fn fetch_by_id_returns_archived_posts_too() {
        let user_id = Uuid::new_v4();
        let mut m = model(user_id, None);
        m.is_deleted = true;

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![m]])
            .into_connection();

        assert!(repo(db)
            .fetch_by_id(Uuid::new_v4())
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn patch_reports_not_found_for_a_missing_post() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<PostModel>::new()])
            .into_connection();

        let err = repo(db)
            .patch(Uuid::new_v4(), PatchBlogPostData::default())
            .await
            .unwrap_err();

        assert!(matches!(err, BlogPostRepositoryError::NotFound));
    }

    /// Unpublishing is a write of NULL, not an absence of input. This is the
    /// case `Option` alone could not express and `BlogPatchField` exists for.
    #[tokio::test]
    async fn patch_can_unpublish_by_writing_null() {
        let user_id = Uuid::new_v4();
        let published = model(user_id, Some(Utc::now()));
        let unpublished = model(user_id, None);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![published]])
            .append_query_results(vec![vec![unpublished]])
            .into_connection();

        let post = repo(db)
            .patch(
                Uuid::new_v4(),
                PatchBlogPostData {
                    published_at: BlogPatchField::Null,
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(post.published_at, None);
    }
}
