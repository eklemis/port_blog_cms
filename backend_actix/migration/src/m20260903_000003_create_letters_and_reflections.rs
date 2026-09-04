//! A cover letter and a reflection, at most one of each per application.
//!
//! Both are keyed on `application_id` as the primary key rather than carrying
//! their own: there is one letter per application and one reflection per
//! application, and expressing that in the schema means no code has to decide
//! which of two rows is the real one.
//!
//! # Reflections hold the most sensitive data in this product
//!
//! Someone's private account of why they think they did not get a job. The
//! rules governing what may be done with it are in
//! `docs/adr/0009-reflections-never-feed-generation.md`; the schema's part is
//! to keep it in its own table, so that nothing reading an application picks it
//! up incidentally.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
            CREATE TABLE IF NOT EXISTS cover_letters (
                application_id UUID PRIMARY KEY
                               REFERENCES applications(id) ON DELETE CASCADE,
                user_id        UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                content        TEXT        NOT NULL DEFAULT '',
                -- The letter's own language, not the writer's interface
                -- language. See m20260903_000002.
                language       TEXT        NOT NULL DEFAULT 'en',
                status         TEXT        NOT NULL DEFAULT 'draft',
                created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
                updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
                CONSTRAINT cover_letters_status_check CHECK (status IN ('draft', 'sent'))
            );
            "#,
        )
        .await?;

        db.execute_unprepared(
            r#"
            CREATE TABLE IF NOT EXISTS reflections (
                application_id UUID PRIMARY KEY
                               REFERENCES applications(id) ON DELETE CASCADE,
                user_id        UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                stage_reached  TEXT        NOT NULL DEFAULT '',
                what_happened  TEXT        NOT NULL DEFAULT '',
                what_id_change TEXT        NOT NULL DEFAULT '',
                created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
                updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
            );
            "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("DROP TABLE IF EXISTS reflections;")
            .await?;
        db.execute_unprepared("DROP TABLE IF EXISTS cover_letters;")
            .await?;
        Ok(())
    }
}
