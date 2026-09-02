//! Adds `bio` to `users`.
//!
//! A public page keyed on `{username}` had no way to introduce the person whose
//! work it was showing: display name, bio and avatar all lived per-CV, and a
//! reader's URL does not say which CV.
//!
//! `bio` becomes a property of the person rather than of one document. The
//! avatar needs no column — it is already modelled by `media_attachments` with
//! `attachable_type = 'user'` and `role = 'avatar'`, both of which the upload
//! path has always accepted.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Nullable with no default, so this is additive: every existing row
        // stays valid and no backfill is needed. That also keeps it safe under
        // the migrate-before-deploy ordering in ADR 0003 — the running build
        // simply ignores a column it does not know about.
        manager
            .get_connection()
            .execute_unprepared(r#"ALTER TABLE users ADD COLUMN IF NOT EXISTS bio TEXT;"#)
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Destructive in the sense that any bio text is lost, which is why the
        // forward migration is the one that matters. Reversible in structure.
        manager
            .get_connection()
            .execute_unprepared(r#"ALTER TABLE users DROP COLUMN IF EXISTS bio;"#)
            .await?;

        Ok(())
    }
}
