//! Two different settings that are easy to confuse.
//!
//! `users.locale` is the language of the **interface** — a property of a
//! person. `resumes.language` and, later, a cover letter's, are the language of
//! a **document**.
//!
//! Conflating them breaks the obvious real case: someone reading the interface
//! in Indonesian while writing an English CV for an international employer. One
//! setting cannot serve both, so there are two.
//!
//! Content language is stored, never inferred. Guessing from existing text
//! breaks on a half-written document, and on CVs that legitimately mix
//! languages — an English technical-skills section inside an Indonesian CV is
//! normal, not a mistake to correct.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Defaulted rather than nullable: every person reads the interface in
        // some language, so "unset" is not a state the UI can render. A CV's
        // language defaults the same way for the same reason.
        db.execute_unprepared(
            r#"ALTER TABLE users ADD COLUMN IF NOT EXISTS locale TEXT NOT NULL DEFAULT 'en';"#,
        )
        .await?;

        db.execute_unprepared(
            r#"ALTER TABLE resumes ADD COLUMN IF NOT EXISTS language TEXT NOT NULL DEFAULT 'en';"#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(r#"ALTER TABLE resumes DROP COLUMN IF EXISTS language;"#)
            .await?;
        db.execute_unprepared(r#"ALTER TABLE users DROP COLUMN IF EXISTS locale;"#)
            .await?;
        Ok(())
    }
}
