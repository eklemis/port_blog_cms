pub use sea_orm_migration::prelude::*;

mod m20210304_000001_create_users_table;
mod m20220101_000010_create_resume_table;
mod m20260127_144214_create_table_topics;
mod m20260127_144229_create_table_projects;
mod m20260127_144248_create_table_project_topics;
mod m20260202_230522_create_table_media;
mod m20260202_231146_create_table_media_attachments;
mod m20260202_231525_create_table_media_variants;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20210304_000001_create_users_table::Migration),
            Box::new(m20220101_000010_create_resume_table::Migration),
            Box::new(m20260127_144214_create_table_topics::Migration),
            Box::new(m20260127_144229_create_table_projects::Migration),
            Box::new(m20260127_144248_create_table_project_topics::Migration),
            Box::new(m20260202_230522_create_table_media::Migration),
            Box::new(m20260202_231146_create_table_media_attachments::Migration),
            Box::new(m20260202_231525_create_table_media_variants::Migration),
        ]
    }
}
