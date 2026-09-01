use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, FromQueryResult, JoinType,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::blog::adapter::outgoing::sea_orm_entity::{
    blog_post_topics,
    blog_posts::{Column as PostColumn, Entity as PostEntity},
};
use crate::blog::application::ports::outgoing::{
    BlogPageRequest, BlogPageResult, BlogPostCard, BlogPostListFilter, BlogPostQuery,
    BlogPostQueryError, BlogPostSort, BlogPostView,
};
use crate::blog::domain::entities::BlogPostTopic;
use crate::topic::adapter::outgoing::sea_orm_entity::topics;

/// The SeaORM implementation of the matching outgoing port.
#[derive(Clone)]
pub struct BlogPostQueryPostgres {
    db: Arc<DatabaseConnection>,
}

/// Exactly the columns a listing row needs. Selecting the full model and
/// discarding `content` would still pull the largest column out of the
/// database for every row on every page.
#[derive(Debug, FromQueryResult)]
struct CardRow {
    id: Uuid,
    title: String,
    slug: String,
    excerpt: Option<String>,
    published_at: Option<sea_orm::prelude::DateTimeWithTimeZone>,
    created_at: sea_orm::prelude::DateTimeWithTimeZone,
    updated_at: sea_orm::prelude::DateTimeWithTimeZone,
}

#[derive(Debug, FromQueryResult)]
struct TopicRow {
    id: Uuid,
    title: String,
    /// Nullable in the topics table, so it must be read as an Option or the
    /// row fails to deserialise on any topic without a description.
    description: Option<String>,
}

impl BlogPostQueryPostgres {
    /// Builds it from the ports it depends on.
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    fn db_err(e: sea_orm::DbErr) -> BlogPostQueryError {
        BlogPostQueryError::DatabaseError(e.to_string())
    }

    /// Shared listing pipeline.
    ///
    /// `published_only` is a separate argument rather than part of the filter,
    /// so the public endpoint cannot accidentally be talked into returning
    /// drafts by whatever the caller put in the query string.
    async fn list(
        &self,
        owner: UserId,
        filter: BlogPostListFilter,
        sort: BlogPostSort,
        page: BlogPageRequest,
        published_only: bool,
    ) -> Result<BlogPageResult<BlogPostCard>, BlogPostQueryError> {
        let mut condition = Condition::all()
            .add(PostColumn::UserId.eq(owner.value()))
            .add(PostColumn::IsDeleted.eq(false));

        if published_only {
            // A future published_at is scheduled, not live, so this is a
            // comparison rather than a NULL check.
            condition = condition
                .add(PostColumn::PublishedAt.is_not_null())
                .add(PostColumn::PublishedAt.lte(Utc::now()));
        } else if let Some(published) = filter.published {
            condition = if published {
                condition.add(PostColumn::PublishedAt.is_not_null())
            } else {
                condition.add(PostColumn::PublishedAt.is_null())
            };
        }

        if let Some(search) = filter
            .search
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            let pattern = format!("%{search}%");
            condition = condition.add(
                Condition::any()
                    .add(PostColumn::Title.like(&pattern))
                    .add(PostColumn::Excerpt.like(&pattern)),
            );
        }

        let mut query = PostEntity::find().filter(condition);

        if let Some(topic_id) = filter.topic_id {
            query = query
                .join(
                    JoinType::InnerJoin,
                    crate::blog::adapter::outgoing::sea_orm_entity::blog_posts::Relation::BlogPostTopics.def(),
                )
                .filter(blog_post_topics::Column::TopicId.eq(topic_id));
        }

        query = match sort {
            BlogPostSort::Newest => query.order_by_desc(PostColumn::CreatedAt),
            BlogPostSort::Oldest => query.order_by_asc(PostColumn::CreatedAt),
            BlogPostSort::RecentlyPublished => query.order_by_desc(PostColumn::PublishedAt),
            BlogPostSort::RecentlyUpdated => query.order_by_desc(PostColumn::UpdatedAt),
        };

        let per_page = page.per_page.clamp(1, 100) as u64;
        let page_number = page.page.max(1) as u64;

        let paginator = query
            .select_only()
            .column(PostColumn::Id)
            .column(PostColumn::Title)
            .column(PostColumn::Slug)
            .column(PostColumn::Excerpt)
            .column(PostColumn::PublishedAt)
            .column(PostColumn::CreatedAt)
            .column(PostColumn::UpdatedAt)
            .into_model::<CardRow>()
            .paginate(&*self.db, per_page);

        let total = paginator.num_items().await.map_err(Self::db_err)?;

        let rows = paginator
            .fetch_page(page_number - 1)
            .await
            .map_err(Self::db_err)?;

        let items = rows
            .into_iter()
            .map(|m| BlogPostCard {
                id: m.id,
                title: m.title,
                slug: m.slug,
                excerpt: m.excerpt,
                published_at: m.published_at.map(|t| t.with_timezone(&Utc)),
                created_at: m.created_at.with_timezone(&Utc),
                updated_at: m.updated_at.with_timezone(&Utc),
            })
            .collect();

        Ok(BlogPageResult {
            items,
            page: page_number as u32,
            per_page: per_page as u32,
            total,
        })
    }

    async fn topics_for(&self, post_id: Uuid) -> Result<Vec<BlogPostTopic>, BlogPostQueryError> {
        // Driven from the join table rather than from `topics`, so the
        // dependency runs blog -> topic. Adding a blog relation to the topic
        // entity would point it the wrong way.
        let rows = blog_post_topics::Entity::find()
            .filter(blog_post_topics::Column::BlogPostId.eq(post_id))
            .join(
                JoinType::InnerJoin,
                blog_post_topics::Relation::Topics.def(),
            )
            .filter(topics::Column::IsDeleted.eq(false))
            .select_only()
            .column(topics::Column::Id)
            .column(topics::Column::Title)
            .column(topics::Column::Description)
            .into_model::<TopicRow>()
            .all(&*self.db)
            .await
            .map_err(Self::db_err)?;

        Ok(rows
            .into_iter()
            .map(|r| BlogPostTopic {
                id: r.id,
                title: r.title,
                description: r.description.unwrap_or_default(),
            })
            .collect())
    }
}

#[async_trait]
impl BlogPostQuery for BlogPostQueryPostgres {
    async fn list_by_owner(
        &self,
        owner: UserId,
        filter: BlogPostListFilter,
        sort: BlogPostSort,
        page: BlogPageRequest,
    ) -> Result<BlogPageResult<BlogPostCard>, BlogPostQueryError> {
        self.list(owner, filter, sort, page, false).await
    }

    async fn list_published(
        &self,
        owner: UserId,
        filter: BlogPostListFilter,
        sort: BlogPostSort,
        page: BlogPageRequest,
    ) -> Result<BlogPageResult<BlogPostCard>, BlogPostQueryError> {
        self.list(owner, filter, sort, page, true).await
    }

    async fn get_by_id(
        &self,
        owner: UserId,
        post_id: Uuid,
    ) -> Result<BlogPostView, BlogPostQueryError> {
        let model = PostEntity::find_by_id(post_id)
            .filter(PostColumn::UserId.eq(owner.value()))
            .filter(PostColumn::IsDeleted.eq(false))
            .one(&*self.db)
            .await
            .map_err(Self::db_err)?
            .ok_or(BlogPostQueryError::NotFound)?;

        let topics = self.topics_for(post_id).await?;

        Ok(BlogPostView {
            post: model.to_domain(),
            topics,
        })
    }

    async fn get_published_by_slug(
        &self,
        owner: UserId,
        slug: &str,
    ) -> Result<BlogPostView, BlogPostQueryError> {
        let model = PostEntity::find()
            .filter(PostColumn::UserId.eq(owner.value()))
            .filter(PostColumn::Slug.eq(slug.trim().to_lowercase()))
            .filter(PostColumn::IsDeleted.eq(false))
            .filter(PostColumn::PublishedAt.is_not_null())
            .filter(PostColumn::PublishedAt.lte(Utc::now()))
            .one(&*self.db)
            .await
            .map_err(Self::db_err)?
            .ok_or(BlogPostQueryError::NotFound)?;

        let post_id = model.id;
        let topics = self.topics_for(post_id).await?;

        Ok(BlogPostView {
            post: model.to_domain(),
            topics,
        })
    }

    async fn get_topics(&self, post_id: Uuid) -> Result<Vec<BlogPostTopic>, BlogPostQueryError> {
        self.topics_for(post_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blog::adapter::outgoing::sea_orm_entity::blog_posts::Model as PostModel;
    use sea_orm::{DatabaseBackend, MockDatabase, Value};
    use std::collections::BTreeMap;

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

    fn count_row(n: i64) -> BTreeMap<String, Value> {
        let mut m = BTreeMap::new();
        m.insert("num_items".to_string(), Value::BigInt(Some(n)));
        m
    }

    fn query(db: DatabaseConnection) -> BlogPostQueryPostgres {
        BlogPostQueryPostgres::new(Arc::new(db))
    }

    #[tokio::test]
    async fn list_by_owner_paginates_and_reports_the_total() {
        let user_id = Uuid::new_v4();
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![count_row(7)]])
            .append_query_results(vec![vec![model(user_id, None), model(user_id, None)]])
            .into_connection();

        let result = query(db)
            .list_by_owner(
                UserId::from(user_id),
                BlogPostListFilter::default(),
                BlogPostSort::Newest,
                BlogPageRequest {
                    page: 1,
                    per_page: 2,
                },
            )
            .await
            .unwrap();

        assert_eq!(result.total, 7);
        assert_eq!(result.per_page, 2);
        assert_eq!(result.items.len(), 2);
    }

    /// A listing row must not carry `content`. It is the largest column and is
    /// never needed to render an index, so BlogPostCard has no field for it.
    #[tokio::test]
    async fn listing_rows_do_not_select_content() {
        let user_id = Uuid::new_v4();
        let conn = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results(vec![vec![count_row(1)]])
                .append_query_results(vec![vec![model(user_id, None)]])
                .into_connection(),
        );

        let q = BlogPostQueryPostgres::new(Arc::clone(&conn));
        q.list_by_owner(
            UserId::from(user_id),
            BlogPostListFilter::default(),
            BlogPostSort::Newest,
            BlogPageRequest::default(),
        )
        .await
        .unwrap();
        drop(q);

        let log = Arc::try_unwrap(conn)
            .expect("sole owner")
            .into_transaction_log();
        let sql = format!("{log:?}").replace("\\\"", "\"");

        assert!(sql.contains("blog_posts.\"title\"") || sql.contains("\"title\""));
        assert!(
            !sql.contains("\"content\""),
            "content must not be selected for a listing: {sql}"
        );
    }

    /// `published` in the filter is honoured for owner listings, which is how
    /// an author lists their drafts.
    #[tokio::test]
    async fn owner_listing_can_ask_for_drafts_only() {
        let user_id = Uuid::new_v4();
        let conn = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results(vec![vec![count_row(0)]])
                .append_query_results(vec![Vec::<PostModel>::new()])
                .into_connection(),
        );

        let q = BlogPostQueryPostgres::new(Arc::clone(&conn));
        q.list_by_owner(
            UserId::from(user_id),
            BlogPostListFilter {
                published: Some(false),
                ..Default::default()
            },
            BlogPostSort::Newest,
            BlogPageRequest::default(),
        )
        .await
        .unwrap();
        drop(q);

        let log = Arc::try_unwrap(conn)
            .expect("sole owner")
            .into_transaction_log();
        let sql = format!("{log:?}").replace("\\\"", "\"");
        assert!(
            sql.contains("published_at\" IS NULL"),
            "drafts-only should filter on a null publish date: {sql}"
        );
    }

    /// The public listing forces published-only regardless of what the filter
    /// says, so a crafted query string cannot surface drafts.
    #[tokio::test]
    async fn public_listing_ignores_a_filter_asking_for_drafts() {
        let user_id = Uuid::new_v4();
        let conn = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results(vec![vec![count_row(0)]])
                .append_query_results(vec![Vec::<PostModel>::new()])
                .into_connection(),
        );

        let q = BlogPostQueryPostgres::new(Arc::clone(&conn));
        q.list_published(
            UserId::from(user_id),
            BlogPostListFilter {
                published: Some(false), // caller asks for drafts
                ..Default::default()
            },
            BlogPostSort::RecentlyPublished,
            BlogPageRequest::default(),
        )
        .await
        .unwrap();
        drop(q);

        let log = Arc::try_unwrap(conn)
            .expect("sole owner")
            .into_transaction_log();
        let sql = format!("{log:?}").replace("\\\"", "\"");
        assert!(
            sql.contains("IS NOT NULL"),
            "public listing must require a publish date: {sql}"
        );
        assert!(
            !sql.contains("published_at\" IS NULL"),
            "public listing must not honour a drafts-only filter: {sql}"
        );
    }

    #[tokio::test]
    async fn get_by_id_reports_not_found_when_absent() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<PostModel>::new()])
            .into_connection();

        let err = query(db)
            .get_by_id(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap_err();

        assert!(matches!(err, BlogPostQueryError::NotFound));
    }

    #[tokio::test]
    async fn get_published_by_slug_reports_not_found_for_a_draft() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<PostModel>::new()])
            .into_connection();

        let err = query(db)
            .get_published_by_slug(UserId::from(Uuid::new_v4()), "hello")
            .await
            .unwrap_err();

        assert!(matches!(err, BlogPostQueryError::NotFound));
    }

    /// Slugs are stored lowercased, so the lookup must normalise too or a
    /// mixed-case URL would 404 against a post that exists.
    #[tokio::test]
    async fn get_published_by_slug_normalises_the_slug() {
        let conn = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results(vec![Vec::<PostModel>::new()])
                .into_connection(),
        );

        let q = BlogPostQueryPostgres::new(Arc::clone(&conn));
        let _ = q
            .get_published_by_slug(UserId::from(Uuid::new_v4()), "  HeLLo  ")
            .await;
        drop(q);

        let log = Arc::try_unwrap(conn)
            .expect("sole owner")
            .into_transaction_log();
        let sql = format!("{log:?}").replace("\\\"", "\"");
        assert!(sql.contains("hello"), "slug should be normalised: {sql}");
        assert!(
            !sql.contains("HeLLo"),
            "raw slug should not be queried: {sql}"
        );
    }

    fn topic_row(title: &str) -> BTreeMap<String, Value> {
        let mut m = BTreeMap::new();
        m.insert("id".into(), Value::Uuid(Some(Box::new(Uuid::new_v4()))));
        m.insert("title".into(), Value::String(Some(Box::new(title.into()))));
        m.insert(
            "description".into(),
            Value::String(Some(Box::new("desc".into()))),
        );
        m
    }

    /// `description` is nullable in the topics table, so the row type reads it
    /// as an Option. A NULL must surface as an empty string, not a
    /// deserialisation failure.
    fn topic_row_without_description() -> BTreeMap<String, Value> {
        let mut m = BTreeMap::new();
        m.insert("id".into(), Value::Uuid(Some(Box::new(Uuid::new_v4()))));
        m.insert("title".into(), Value::String(Some(Box::new("Rust".into()))));
        m.insert("description".into(), Value::String(None));
        m
    }

    #[tokio::test]
    async fn get_by_id_returns_the_post_with_its_topics() {
        let user_id = Uuid::new_v4();
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![model(user_id, None)]])
            .append_query_results(vec![vec![topic_row("Rust"), topic_row("Actix")]])
            .into_connection();

        let view = query(db)
            .get_by_id(UserId::from(user_id), Uuid::new_v4())
            .await
            .unwrap();

        assert_eq!(view.post.slug, "hello");
        assert_eq!(view.topics.len(), 2);
        assert_eq!(view.topics[0].title, "Rust");
    }

    #[tokio::test]
    async fn a_topic_with_no_description_reads_as_empty_rather_than_failing() {
        let user_id = Uuid::new_v4();
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![model(user_id, None)]])
            .append_query_results(vec![vec![topic_row_without_description()]])
            .into_connection();

        let view = query(db)
            .get_by_id(UserId::from(user_id), Uuid::new_v4())
            .await
            .unwrap();

        assert_eq!(view.topics[0].description, "");
    }

    #[tokio::test]
    async fn get_published_by_slug_returns_a_published_post() {
        let user_id = Uuid::new_v4();
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![model(user_id, Some(Utc::now()))]])
            .append_query_results(vec![Vec::<BTreeMap<String, Value>>::new()])
            .into_connection();

        let view = query(db)
            .get_published_by_slug(UserId::from(user_id), "hello")
            .await
            .unwrap();

        assert!(view.post.published_at.is_some());
        assert!(view.topics.is_empty());
    }

    #[tokio::test]
    async fn get_topics_returns_the_attached_topics() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![topic_row("Rust")]])
            .into_connection();

        let topics = query(db).get_topics(Uuid::new_v4()).await.unwrap();
        assert_eq!(topics.len(), 1);
        assert_eq!(topics[0].title, "Rust");
    }

    /// A blank or whitespace-only search must not become a `LIKE '%%'` filter,
    /// which would be a no-op clause on every listing query.
    #[tokio::test]
    async fn a_blank_search_term_is_ignored() {
        let user_id = Uuid::new_v4();
        let conn = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results(vec![vec![count_row(0)]])
                .append_query_results(vec![Vec::<BTreeMap<String, Value>>::new()])
                .into_connection(),
        );

        let q = BlogPostQueryPostgres::new(Arc::clone(&conn));
        q.list_by_owner(
            UserId::from(user_id),
            BlogPostListFilter {
                search: Some("   ".into()),
                ..Default::default()
            },
            BlogPostSort::Newest,
            BlogPageRequest::default(),
        )
        .await
        .unwrap();
        drop(q);

        let sql = format!(
            "{:?}",
            Arc::try_unwrap(conn)
                .expect("sole owner")
                .into_transaction_log()
        )
        .replace("\\\"", "\"");
        assert!(
            !sql.contains("LIKE"),
            "blank search should add no clause: {sql}"
        );
    }

    #[tokio::test]
    async fn a_search_term_filters_on_title_and_excerpt() {
        let user_id = Uuid::new_v4();
        let conn = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results(vec![vec![count_row(0)]])
                .append_query_results(vec![Vec::<BTreeMap<String, Value>>::new()])
                .into_connection(),
        );

        let q = BlogPostQueryPostgres::new(Arc::clone(&conn));
        q.list_by_owner(
            UserId::from(user_id),
            BlogPostListFilter {
                search: Some("  rust  ".into()),
                ..Default::default()
            },
            BlogPostSort::Newest,
            BlogPageRequest::default(),
        )
        .await
        .unwrap();
        drop(q);

        let sql = format!(
            "{:?}",
            Arc::try_unwrap(conn)
                .expect("sole owner")
                .into_transaction_log()
        )
        .replace("\\\"", "\"");
        assert!(sql.contains("LIKE"));
        // Trimmed before being wrapped in wildcards.
        assert!(sql.contains("%rust%"), "{sql}");
        assert!(sql.contains("title") && sql.contains("excerpt"));
    }

    #[tokio::test]
    async fn a_topic_filter_joins_the_link_table() {
        let user_id = Uuid::new_v4();
        let conn = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results(vec![vec![count_row(0)]])
                .append_query_results(vec![Vec::<BTreeMap<String, Value>>::new()])
                .into_connection(),
        );

        let q = BlogPostQueryPostgres::new(Arc::clone(&conn));
        q.list_by_owner(
            UserId::from(user_id),
            BlogPostListFilter {
                topic_id: Some(Uuid::new_v4()),
                ..Default::default()
            },
            BlogPostSort::Newest,
            BlogPageRequest::default(),
        )
        .await
        .unwrap();
        drop(q);

        let sql = format!(
            "{:?}",
            Arc::try_unwrap(conn)
                .expect("sole owner")
                .into_transaction_log()
        )
        .replace("\\\"", "\"");
        assert!(sql.contains("blog_post_topics"), "{sql}");
    }

    /// Each sort maps to a distinct ORDER BY. Getting one wrong silently
    /// reorders a public index, which no other test would notice.
    #[tokio::test]
    async fn every_sort_variant_maps_to_its_column() {
        for (sort, col, dir) in [
            (BlogPostSort::Newest, "created_at", "DESC"),
            (BlogPostSort::Oldest, "created_at", "ASC"),
            (BlogPostSort::RecentlyPublished, "published_at", "DESC"),
            (BlogPostSort::RecentlyUpdated, "updated_at", "DESC"),
        ] {
            let conn = Arc::new(
                MockDatabase::new(DatabaseBackend::Postgres)
                    .append_query_results(vec![vec![count_row(0)]])
                    .append_query_results(vec![Vec::<BTreeMap<String, Value>>::new()])
                    .into_connection(),
            );

            let q = BlogPostQueryPostgres::new(Arc::clone(&conn));
            q.list_by_owner(
                UserId::from(Uuid::new_v4()),
                BlogPostListFilter::default(),
                sort,
                BlogPageRequest::default(),
            )
            .await
            .unwrap();
            drop(q);

            let sql = format!(
                "{:?}",
                Arc::try_unwrap(conn)
                    .expect("sole owner")
                    .into_transaction_log()
            )
            .replace("\\\"", "\"");
            assert!(
                sql.contains(&format!("{col}\" {dir}")),
                "expected ORDER BY {col} {dir}, got: {sql}"
            );
        }
    }

    /// per_page is clamped, so a caller cannot ask for an unbounded page and
    /// pull the whole table in one request.
    #[tokio::test]
    async fn per_page_is_clamped_and_page_is_at_least_one() {
        let conn = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results(vec![vec![count_row(0)]])
                .append_query_results(vec![Vec::<BTreeMap<String, Value>>::new()])
                .into_connection(),
        );

        let q = BlogPostQueryPostgres::new(Arc::clone(&conn));
        let result = q
            .list_by_owner(
                UserId::from(Uuid::new_v4()),
                BlogPostListFilter::default(),
                BlogPostSort::Newest,
                BlogPageRequest {
                    page: 0,
                    per_page: 10_000,
                },
            )
            .await
            .unwrap();

        assert_eq!(result.page, 1, "page 0 should become page 1");
        assert_eq!(result.per_page, 100, "per_page should clamp to 100");
    }

    #[tokio::test]
    async fn database_errors_are_surfaced() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors([sea_orm::DbErr::Custom("db down".into())])
            .into_connection();

        let err = query(db).get_topics(Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, BlogPostQueryError::DatabaseError(m) if m.contains("db down")));
    }
}
