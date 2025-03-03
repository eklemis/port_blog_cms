use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{json, string};

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
                    .col(ColumnDef::new(Cv::UserId).uuid().not_null().primary_key())
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
    UserId,
    Bio,
    PhotoUrl,
    EducationsJson,
    ExperiencesJson,
    HighlightedProjectsJson,
    CreatedAt,
    UpdatedAt,
}
