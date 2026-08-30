use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // =====================================================
        // Create blog_posts table
        //
        // Publication state is carried by `published_at` rather than a status
        // enum: NULL is a draft, a timestamp is published. That keeps the
        // publish date — which a blog needs regardless — and the ordering key
        // for public listings in one column, with no extra enum type to
        // migrate when states are added.
        // =====================================================
        manager
            .create_table(
                Table::create()
                    .table(BlogPosts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BlogPosts::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("gen_random_uuid()")),
                    )
                    .col(ColumnDef::new(BlogPosts::UserId).uuid().not_null())
                    .col(ColumnDef::new(BlogPosts::Title).string_len(200).not_null())
                    .col(ColumnDef::new(BlogPosts::Slug).string_len(200).not_null())
                    .col(ColumnDef::new(BlogPosts::Excerpt).text())
                    .col(ColumnDef::new(BlogPosts::Content).text().not_null())
                    .col(ColumnDef::new(BlogPosts::PublishedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(BlogPosts::IsDeleted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(BlogPosts::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(BlogPosts::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_blog_posts_user_id")
                            .from(BlogPosts::Table, BlogPosts::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Fast lookup by author
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX IF NOT EXISTS idx_blog_posts_user_id
                ON blog_posts (user_id);
                "#,
            )
            .await?;

        // Slug is unique PER AUTHOR, not globally.
        //
        // Public posts are addressed as /api/public/blog/{username}/{slug}, so
        // the username already disambiguates. A global unique index — as
        // `projects` has — would stop two authors from both publishing a post
        // slugged "hello-world", which is not a constraint the URL scheme
        // implies.
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE UNIQUE INDEX IF NOT EXISTS idx_blog_posts_user_slug_unique
                ON blog_posts (user_id, lower(slug));
                "#,
            )
            .await?;

        // Public listings read published posts newest-first. Partial, because
        // drafts are never listed publicly and would only bloat the index.
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX IF NOT EXISTS idx_blog_posts_published_at
                ON blog_posts (published_at DESC)
                WHERE published_at IS NOT NULL AND is_deleted = false;
                "#,
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TRIGGER update_blog_posts_updated_at
                BEFORE UPDATE ON blog_posts
                FOR EACH ROW
                EXECUTE FUNCTION update_updated_at_column();
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP TRIGGER IF EXISTS update_blog_posts_updated_at ON blog_posts;
                DROP INDEX IF EXISTS idx_blog_posts_published_at;
                DROP INDEX IF EXISTS idx_blog_posts_user_slug_unique;
                DROP INDEX IF EXISTS idx_blog_posts_user_id;
                "#,
            )
            .await?;

        manager
            .drop_table(Table::drop().table(BlogPosts::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum BlogPosts {
    Table,
    Id,
    UserId,
    Title,
    Slug,
    Excerpt,
    Content,
    PublishedAt,
    IsDeleted,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
