use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // =====================================================
        // Create projects table
        // =====================================================
        manager
            .create_table(
                Table::create()
                    .table(Projects::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Projects::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("gen_random_uuid()")),
                    )
                    .col(ColumnDef::new(Projects::UserId).uuid().not_null())
                    .col(ColumnDef::new(Projects::Title).string_len(150).not_null())
                    .col(ColumnDef::new(Projects::Slug).string_len(150).not_null())
                    .col(ColumnDef::new(Projects::Description).text().not_null())
                    .col(ColumnDef::new(Projects::TechStack).json_binary().not_null())
                    .col(
                        ColumnDef::new(Projects::Screenshots)
                            .json_binary()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Projects::RepoUrl).text())
                    .col(ColumnDef::new(Projects::LiveDemoUrl).text())
                    .col(
                        ColumnDef::new(Projects::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Projects::IsDeleted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Projects::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_projects_user_id")
                            .from(Projects::Table, Projects::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // =====================================================
        // Indexes
        // =====================================================

        // Fast lookup by user
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX IF NOT EXISTS idx_projects_user_id
                ON projects (user_id);
                "#,
            )
            .await?;

        // Enforce GLOBAL slug uniqueness (case-insensitive)
        // Using lower(slug) avoids Rust/rust collisions without needing citext.
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE UNIQUE INDEX IF NOT EXISTS idx_projects_slug_unique
                ON projects (lower(slug));
                "#,
            )
            .await?;
        // GIN index for fast containment queries
        // Handle fast retrieval for query like "find projects with tech X" or `SELECT * FROM projects WHERE tech_stack @> '["Rust"]';`
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX IF NOT EXISTS idx_projects_tech_stack
                ON projects USING GIN (tech_stack);
                "#,
            )
            .await?;

        // =====================================================
        // updated_at trigger
        // =====================================================

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TRIGGER update_projects_updated_at
                BEFORE UPDATE ON projects
                FOR EACH ROW
                EXECUTE FUNCTION update_updated_at_column();
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop trigger
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP TRIGGER IF EXISTS update_projects_updated_at ON projects;
                "#,
            )
            .await?;

        // Drop indexes
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP INDEX IF EXISTS idx_projects_user_id;
                DROP INDEX IF EXISTS idx_projects_slug_unique;
                "#,
            )
            .await?;

        // Drop table
        manager
            .drop_table(Table::drop().table(Projects::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    Id,
    UserId,
    Title,
    Slug,
    Description,
    TechStack,
    Screenshots,
    RepoUrl,
    LiveDemoUrl,
    IsDeleted,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
