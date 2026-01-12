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
                    .table(Cv::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Cv::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_owned()),
                    )
                    .col(uuid(Cv::UserId).not_null())
                    .col(string(Cv::Bio).not_null())
                    .col(string(Cv::PhotoUrl).not_null())
                    .col(json(Cv::EducationsJson).not_null())
                    .col(json(Cv::ExperiencesJson).not_null())
                    .col(json(Cv::HighlightedProjectsJson).not_null())
                    .col(
                        ColumnDef::new(Cv::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(Cv::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()".to_owned()),
                    )
                    .to_owned(),
            )
            .await?;

        // Add index on UserId for better query performance
        manager
            .create_index(
                Index::create()
                    .table(Cv::Table)
                    .name("idx_cv_user_id")
                    .col(Cv::UserId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Cv::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Cv {
    Table,
    Id,
    UserId,
    Bio,
    PhotoUrl,
    EducationsJson,
    ExperiencesJson,
    HighlightedProjectsJson,
    CreatedAt,
    UpdatedAt,
}
