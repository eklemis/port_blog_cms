use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // =====================================================
        // Create project_topics join table
        // =====================================================
        manager
            .create_table(
                Table::create()
                    .table(ProjectTopics::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ProjectTopics::ProjectId).uuid().not_null())
                    .col(ColumnDef::new(ProjectTopics::TopicId).uuid().not_null())
                    .col(
                        ColumnDef::new(ProjectTopics::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    // Composite primary key
                    .primary_key(
                        Index::create()
                            .col(ProjectTopics::ProjectId)
                            .col(ProjectTopics::TopicId),
                    )
                    // FK → projects
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_project_topics_project_id")
                            .from(ProjectTopics::Table, ProjectTopics::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    // FK → topics
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_project_topics_topic_id")
                            .from(ProjectTopics::Table, ProjectTopics::TopicId)
                            .to(Topics::Table, Topics::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // =====================================================
        // Indexes
        // =====================================================

        // Fast lookup: all projects for a topic
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX idx_project_topics_topic_id
                ON project_topics (topic_id);
                "#,
            )
            .await?;

        // Fast lookup: all topics for a project
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX idx_project_topics_project_id
                ON project_topics (project_id);
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP INDEX IF EXISTS idx_project_topics_topic_id;
                DROP INDEX IF EXISTS idx_project_topics_project_id;
                "#,
            )
            .await?;

        // Drop table
        manager
            .drop_table(Table::drop().table(ProjectTopics::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ProjectTopics {
    Table,
    ProjectId,
    TopicId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Topics {
    Table,
    Id,
}
