use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{json, string, uuid};

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
                    .col(uuid(Resumes::UserId).not_null())
                    .col(string(Resumes::Role).not_null())
                    .col(string(Resumes::Bio).not_null())
                    .col(string(Resumes::PhotoUrl).not_null())
                    .col(json(Resumes::CoreSkills).not_null())
                    .col(json(Resumes::Educations).not_null())
                    .col(json(Resumes::Experiences).not_null())
                    .col(json(Resumes::HighlightedProjects).not_null())
                    .col(
                        ColumnDef::new(Resumes::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .col(
                        ColumnDef::new(Resumes::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .col(
                        ColumnDef::new(Resumes::IsDeleted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(Resumes::Table)
                    .name("idx_cv_user_id")
                    .col(Resumes::UserId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Resumes::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Resumes {
    Table,
    Id,
    UserId,
    Role,
    Bio,
    PhotoUrl,
    CoreSkills,
    Educations,
    Experiences,
    HighlightedProjects,
    CreatedAt,
    UpdatedAt,
    IsDeleted,
}
