use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::auth::application::domain::entities::UserId;
use crate::modules::topic::application::ports::outgoing::{
    TopicQuery, TopicQueryError, TopicQueryResult, TopicUsage,
};
use uuid::Uuid;

// SeaORM entity

/// The SeaORM implementation of the matching outgoing port.
#[derive(Debug, Clone)]
pub struct TopicQueryPostgres {
    db: Arc<DatabaseConnection>,
}

impl TopicQueryPostgres {
    /// The listing statement, with both usage counts computed in it.
    ///
    /// Extracted so a test can read the statement the code actually runs: a
    /// query that returned topics without counts would look identical to every
    /// caller until the column was wired, and a test that asserts on SQL it
    /// wrote itself proves only that the test can type.
    fn list_topics_stmt(owner: UserId) -> sea_orm::Statement {
        sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            r#"
            SELECT
                t.id,
                t.user_id,
                t.title,
                COALESCE(t.description, '') AS description,
                t.created_at,
                t.updated_at,
                (SELECT COUNT(*) FROM blog_post_topics bt
                   JOIN blog_posts p ON p.id = bt.blog_post_id
                  WHERE bt.topic_id = t.id AND p.is_deleted = false) AS post_count,
                (SELECT COUNT(*) FROM project_topics pt
                   JOIN projects pr ON pr.id = pt.project_id
                  WHERE pt.topic_id = t.id AND pr.is_deleted = false) AS project_count
            FROM topics t
            WHERE t.user_id = $1 AND t.is_deleted = false
            ORDER BY t.created_at DESC
            "#,
            vec![owner.value().into()],
        )
    }

    /// Builds it from the ports it depends on.
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl TopicQuery for TopicQueryPostgres {
    async fn get_topic_usage(
        &self,
        owner: UserId,
        topic_id: Uuid,
    ) -> Result<TopicUsage, TopicQueryError> {
        // Two counts in one round trip. Both exclude soft-deleted rows: a
        // deleted post is not a reason to keep a topic alive, and counting it
        // would make the confirmation overstate the damage.
        let stmt = sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            r#"
            SELECT
                (SELECT COUNT(*) FROM blog_post_topics bt
                   JOIN blog_posts p ON p.id = bt.blog_post_id
                  WHERE bt.topic_id = $1 AND p.is_deleted = false) AS posts,
                (SELECT COUNT(*) FROM project_topics pt
                   JOIN projects pr ON pr.id = pt.project_id
                  WHERE pt.topic_id = $1 AND pr.is_deleted = false) AS projects
            FROM topics t
            WHERE t.id = $1 AND t.user_id = $2
            "#,
            vec![topic_id.into(), owner.value().into()],
        );

        let row = sea_orm::ConnectionTrait::query_one(&*self.db, stmt)
            .await
            .map_err(|e| TopicQueryError::DatabaseError(e.to_string()))?;

        // No row means the topic does not exist or belongs to someone else.
        // Reported as zero usage rather than an error: the caller is about to
        // be told the topic is not found by the operation it actually wants.
        let Some(row) = row else {
            return Ok(TopicUsage {
                posts: 0,
                projects: 0,
            });
        };

        let posts: i64 = row
            .try_get("", "posts")
            .map_err(|e| TopicQueryError::DatabaseError(e.to_string()))?;
        let projects: i64 = row
            .try_get("", "projects")
            .map_err(|e| TopicQueryError::DatabaseError(e.to_string()))?;

        Ok(TopicUsage {
            posts: posts.max(0) as u64,
            projects: projects.max(0) as u64,
        })
    }

    async fn get_topics(&self, owner: UserId) -> Result<Vec<TopicQueryResult>, TopicQueryError> {
        // The counts are computed here rather than fetched per row.
        //
        // The screen that lists topics draws how many things use each one, and
        // this listing is unpaged: asking per row is one request per topic on
        // every load, for a list that grows with someone's vocabulary. The two
        // subqueries are the ones `get_topic_usage` runs, so a row's count and
        // the retire confirmation's count come from one definition — including
        // the soft-delete rule, which is the part that would otherwise drift.
        let rows = sea_orm::ConnectionTrait::query_all(&*self.db, Self::list_topics_stmt(owner))
            .await
            .map_err(|e| TopicQueryError::DatabaseError(e.to_string()))?;

        let mut topics = Vec::with_capacity(rows.len());

        for row in rows {
            let err = |e: sea_orm::DbErr| TopicQueryError::DatabaseError(e.to_string());

            topics.push(TopicQueryResult {
                id: row.try_get("", "id").map_err(err)?,
                owner: UserId::from(row.try_get::<Uuid>("", "user_id").map_err(err)?),
                title: row.try_get("", "title").map_err(err)?,
                description: row.try_get("", "description").map_err(err)?,
                created_at: row
                    .try_get::<chrono::DateTime<chrono::FixedOffset>>("", "created_at")
                    .map_err(err)?
                    .into(),
                updated_at: row
                    .try_get::<chrono::DateTime<chrono::FixedOffset>>("", "updated_at")
                    .map_err(err)?
                    .into(),
                // COUNT(*) is bigint, and a count cannot be negative.
                post_count: row.try_get::<i64>("", "post_count").map_err(err)? as u64,
                project_count: row.try_get::<i64>("", "project_count").map_err(err)? as u64,
            });
        }

        Ok(topics)
    }
}

#[cfg(test)]
mod counting_tests {
    use super::*;
    use crate::auth::application::domain::entities::UserId;
    use uuid::Uuid;

    /// The counts come from the listing statement itself.
    ///
    /// The screen draws a usage count per row and the listing is unpaged, so
    /// asking per row is one request per topic on every load.
    ///
    /// Both subqueries carry the soft-delete rule, which is the part that would
    /// drift: the retire confirmation counts the same way, and the two numbers
    /// must not disagree the moment someone archives a post.
    #[test]
    fn the_listing_counts_usage_in_one_statement() {
        let sql = TopicQueryPostgres::list_topics_stmt(UserId::from(Uuid::new_v4())).to_string();

        assert!(
            sql.contains("AS post_count"),
            "posts are counted here: {sql}"
        );
        assert!(
            sql.contains("AS project_count"),
            "projects are counted here: {sql}"
        );
        assert_eq!(
            sql.matches("is_deleted = false").count(),
            3,
            "the rule applies to the topic and to both counts, so a row's \
             number matches the retire confirmation's: {sql}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::domain::entities::UserId;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, DbErr, MockDatabase, RuntimeErr};
    use std::sync::Arc;
    use uuid::Uuid;

    /// A row shaped like the listing statement's own SELECT.
    ///
    /// The mock must answer with the columns the statement asks for. Feeding it
    /// a whole entity was fine while the query was a SeaORM `find()`; now that
    /// the counts are computed in SQL, a row that carries more or fewer columns
    /// than the statement selects is a mock agreeing with itself.
    fn topic_row(
        user_id: Uuid,
        title: &str,
        seconds_ago: i64,
        post_count: i64,
        project_count: i64,
    ) -> std::collections::BTreeMap<String, sea_orm::Value> {
        let at = (Utc::now() - chrono::Duration::seconds(seconds_ago)).fixed_offset();
        let mut row = std::collections::BTreeMap::new();
        row.insert("id".to_string(), sea_orm::Value::from(Uuid::new_v4()));
        row.insert("user_id".to_string(), sea_orm::Value::from(user_id));
        row.insert("title".to_string(), sea_orm::Value::from(title));
        row.insert("description".to_string(), sea_orm::Value::from(""));
        row.insert("created_at".to_string(), sea_orm::Value::from(at));
        row.insert("updated_at".to_string(), sea_orm::Value::from(at));
        row.insert("post_count".to_string(), sea_orm::Value::from(post_count));
        row.insert(
            "project_count".to_string(),
            sea_orm::Value::from(project_count),
        );
        row
    }

    #[tokio::test]
    async fn test_get_topics_success() {
        let user_uuid = Uuid::new_v4();
        let owner = UserId::from(user_uuid);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![
                topic_row(user_uuid, "Topic B", 20, 6, 2),
                topic_row(user_uuid, "Topic A", 10, 3, 0),
            ]])
            .into_connection();

        let query = TopicQueryPostgres::new(Arc::new(db));

        let result = query.get_topics(owner).await;

        assert!(result.is_ok());
        let topics = result.unwrap();

        assert_eq!(topics.len(), 2);

        // Ordered by created_at DESC
        assert_eq!(topics[0].title, "Topic B");
        assert_eq!(topics[1].title, "Topic A");

        assert_eq!(topics[0].owner, owner);

        // The counts travel with the row rather than being fetched per topic.
        assert_eq!((topics[0].post_count, topics[0].project_count), (6, 2));
        assert_eq!((topics[1].post_count, topics[1].project_count), (3, 0));
    }

    #[tokio::test]
    async fn test_get_topics_filters_deleted() {
        let user_uuid = Uuid::new_v4();
        let owner = UserId::from(user_uuid);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // The statement filters deleted rows out; the mock answers with
            // what such a statement would return.
            .append_query_results(vec![vec![topic_row(user_uuid, "Active Topic", 10, 0, 0)]])
            .into_connection();

        let query = TopicQueryPostgres::new(Arc::new(db));

        let result = query.get_topics(owner).await;

        assert!(result.is_ok());
        let topics = result.unwrap();

        assert_eq!(topics.len(), 1);
        assert_eq!(topics[0].title, "Active Topic");
    }

    #[tokio::test]
    async fn test_get_topics_empty_result() {
        let owner = UserId::from(Uuid::new_v4());

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<
                std::collections::BTreeMap<String, sea_orm::Value>,
            >::new()])
            .into_connection();

        let query = TopicQueryPostgres::new(Arc::new(db));

        let result = query.get_topics(owner).await;

        assert!(result.is_ok());
        let topics = result.unwrap();
        assert!(topics.is_empty());
    }

    #[tokio::test]
    async fn test_get_topics_database_error() {
        let owner = UserId::from(Uuid::new_v4());

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors(vec![DbErr::Query(RuntimeErr::Internal(
                "connection lost".into(),
            ))])
            .into_connection();

        let query = TopicQueryPostgres::new(Arc::new(db));

        let result = query.get_topics(owner).await;

        assert!(matches!(result, Err(TopicQueryError::DatabaseError(_))));
    }

    #[test]
    fn test_topic_query_postgres_is_cloneable() {
        let db = MockDatabase::new(DatabaseBackend::Postgres).into_connection();
        let query = TopicQueryPostgres::new(Arc::new(db));

        let _clone = query.clone();
    }
}
