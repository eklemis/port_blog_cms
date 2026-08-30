use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // `idx_projects_slug_unique` was UNIQUE (lower(slug)) — globally unique
        // across every author. But a public project is addressed as
        //
        //     /api/public/projects/{username}/{project_slug}
        //
        // so the username already disambiguates. The global index only meant
        // that once any user had a project slugged "portfolio-site", no other
        // user could, and the second author saw SLUG_ALREADY_EXISTS for a slug
        // they had never used.
        //
        // Rebuilding it as (user_id, lower(slug)) strictly relaxes the
        // constraint: every row permitted by the old index is permitted by the
        // new one, so no existing data can conflict.
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP INDEX IF EXISTS idx_projects_slug_unique;
                CREATE UNIQUE INDEX IF NOT EXISTS idx_projects_user_slug_unique
                ON projects (user_id, lower(slug));
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Reverting can fail where the relaxed index has already allowed two
        // authors to share a slug. That is expected: the old constraint was
        // stricter, so the rollback is only possible while no such pair exists.
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP INDEX IF EXISTS idx_projects_user_slug_unique;
                CREATE UNIQUE INDEX IF NOT EXISTS idx_projects_slug_unique
                ON projects (lower(slug));
                "#,
            )
            .await?;

        Ok(())
    }
}
