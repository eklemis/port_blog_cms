use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // =====================================================
        // Create topics table
        // =====================================================
        manager
            .create_table(
                Table::create()
                    .table(Topics::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Topics::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("gen_random_uuid()")),
                    )
                    .col(ColumnDef::new(Topics::UserId).uuid().not_null())
                    .col(ColumnDef::new(Topics::Title).string_len(100).not_null())
                    .col(ColumnDef::new(Topics::Description).text())
                    .col(
                        ColumnDef::new(Topics::IsDeleted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Topics::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Topics::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_topics_user_id")
                            .from(Topics::Table, Topics::UserId)
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
                CREATE INDEX idx_topics_user_id
                ON topics (user_id);
                "#,
            )
            .await?;

        // Enforce unique topic title per user (case-insensitive)
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE UNIQUE INDEX idx_topics_user_title_unique
                ON topics (user_id, lower(title));
                "#,
            )
            .await?;

        // =====================================================
        // updated_at trigger (reuse same function pattern)
        // =====================================================

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TRIGGER update_topics_updated_at
                BEFORE UPDATE ON topics
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
                DROP TRIGGER IF EXISTS update_topics_updated_at ON topics;
                "#,
            )
            .await?;

        // Drop indexes explicitly
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP INDEX IF EXISTS idx_topics_user_id;
                DROP INDEX IF EXISTS idx_topics_user_title_unique;
                "#,
            )
            .await?;

        // Drop table
        manager
            .drop_table(Table::drop().table(Topics::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Topics {
    Table,
    Id,
    UserId,
    Title,
    Description,
    IsDeleted,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
