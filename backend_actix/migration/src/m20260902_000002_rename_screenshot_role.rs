//! Corrects the `screenshoot` media role to `screenshot`.
//!
//! The migration that created `media_attachments` documents the role as
//! `'screenshot'`, but that comment was never executed — the code wrote
//! `'screenshoot'`, so that is what every row holds.
//!
//! [ADR 0007](../../docs/adr/0007-screenshot-role-rename.md) planned this as
//! the middle of three deploys, because a rename that lands while an older
//! build is still serving breaks every media read. There was no deployed
//! backend when it came to be done, so there was no build to break and
//! [ADR 0008](../../docs/adr/0008-collapse-the-screenshot-rename.md) collapsed
//! the sequence. The reasoning in 0007 still applies to the next rename, when
//! something is running.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Scoped to the misspelling, so running it twice is a no-op and it
        // cannot touch a role it was not meant to.
        manager
            .get_connection()
            .execute_unprepared(
                r#"UPDATE media_attachments SET role = 'screenshot' WHERE role = 'screenshoot';"#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Restores the misspelling. The code still parses it, so a rollback
        // that runs this reads its own rows correctly.
        manager
            .get_connection()
            .execute_unprepared(
                r#"UPDATE media_attachments SET role = 'screenshoot' WHERE role = 'screenshot';"#,
            )
            .await?;

        Ok(())
    }
}
