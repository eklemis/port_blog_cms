use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Resumes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Resumes::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("gen_random_uuid()")),
                    )
                    .col(ColumnDef::new(Resumes::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(Resumes::DisplayName)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Resumes::Role).string_len(100).not_null())
                    .col(ColumnDef::new(Resumes::Bio).text().not_null())
                    .col(ColumnDef::new(Resumes::PhotoUrl).string_len(500).not_null())
                    .col(ColumnDef::new(Resumes::CoreSkills).json_binary().not_null())
                    .col(ColumnDef::new(Resumes::Educations).json_binary().not_null())
                    .col(
                        ColumnDef::new(Resumes::Experiences)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Resumes::HighlightedProjects)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Resumes::ContactInfo)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Resumes::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Resumes::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Resumes::IsDeleted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_resumes_user_id")
                            .from(Resumes::Table, Resumes::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Index for fetching user's active resumes
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX idx_resumes_user_id_active
                ON resumes (user_id, created_at DESC)
                WHERE is_deleted = false;
                "#,
            )
            .await?;

        // Index for soft-delete operations
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX idx_resumes_user_deleted
                ON resumes (user_id, is_deleted);
                "#,
            )
            .await?;

        // Trigger for updated_at
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE OR REPLACE FUNCTION update_updated_at_column()
                RETURNS TRIGGER AS $$
                BEGIN
                    NEW.updated_at = CURRENT_TIMESTAMP;
                    RETURN NEW;
                END;
                $$ language 'plpgsql';
                "#,
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TRIGGER update_resumes_updated_at
                BEFORE UPDATE ON resumes
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
            .execute_unprepared("DROP TRIGGER IF EXISTS update_resumes_updated_at ON resumes")
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP INDEX IF EXISTS idx_resumes_user_id_active;
                DROP INDEX IF EXISTS idx_resumes_user_deleted;
                "#,
            )
            .await?;

        manager
            .drop_table(Table::drop().table(Resumes::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Resumes {
    Table,
    Id,
    DisplayName,
    UserId,
    Role,
    Bio,
    PhotoUrl, //Old Design Decision - Just Ignored
    CoreSkills,
    Educations,
    Experiences,
    HighlightedProjects, //Old Design Decision - Just Ignored
    ContactInfo,
    CreatedAt,
    UpdatedAt,
    IsDeleted,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
