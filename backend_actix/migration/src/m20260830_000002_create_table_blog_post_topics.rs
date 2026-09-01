use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Join table mirroring project_topics, so blog posts reuse the existing
        // topic module rather than growing a parallel tagging concept.
        manager
            .create_table(
                Table::create()
                    .table(BlogPostTopics::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(BlogPostTopics::BlogPostId).uuid().not_null())
                    .col(ColumnDef::new(BlogPostTopics::TopicId).uuid().not_null())
                    .col(
                        ColumnDef::new(BlogPostTopics::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    // Composite primary key makes a post/topic pair unique, so
                    // attaching the same topic twice is a no-op rather than a
                    // duplicate row.
                    .primary_key(
                        Index::create()
                            .col(BlogPostTopics::BlogPostId)
                            .col(BlogPostTopics::TopicId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_blog_post_topics_blog_post_id")
                            .from(BlogPostTopics::Table, BlogPostTopics::BlogPostId)
                            .to(BlogPosts::Table, BlogPosts::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_blog_post_topics_topic_id")
                            .from(BlogPostTopics::Table, BlogPostTopics::TopicId)
                            .to(Topics::Table, Topics::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Reverse lookup: "which posts carry this topic".
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX IF NOT EXISTS idx_blog_post_topics_topic_id
                ON blog_post_topics (topic_id);
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(r#"DROP INDEX IF EXISTS idx_blog_post_topics_topic_id;"#)
            .await?;

        manager
            .drop_table(Table::drop().table(BlogPostTopics::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum BlogPostTopics {
    Table,
    BlogPostId,
    TopicId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum BlogPosts {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Topics {
    Table,
    Id,
}
