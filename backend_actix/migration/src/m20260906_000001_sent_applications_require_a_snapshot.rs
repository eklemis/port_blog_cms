//! Makes "a sent application carries a snapshot" an invariant rather than a
//! convention.
//!
//! The rule already lives in `ApplicationService`, which is where the friendly
//! `SNAPSHOT_REQUIRED` error comes from. But a rule enforced only in a service
//! is decided on a row read moments earlier, and two concurrent transitions can
//! both read a draft, both find the rule satisfied, and both write. It also
//! binds only the one code path that remembers to check.
//!
//! A CHECK constraint binds every path and every race, permanently. The service
//! keeps the readable error; this makes the state unreachable even if it stops.
//!
//! This migration is the case study behind
//! [ADR 0010](../../docs/adr/0010-migrations-must-be-backward-compatible.md).
//! It is additive in DDL terms and still not safe on its own: a constraint is
//! enforced against whatever build is running, including the old one during a
//! deploy. It was safe here only because the service check it backstops had
//! shipped weeks earlier, so the running build had already stopped producing
//! rows that violate it.
//!
//! Existing rows are repaired first: an already-sent application with no
//! snapshot is returned to `draft`, because a sent application pointing at a
//! living CV is exactly the misreport the rule exists to prevent, and there is
//! no snapshot to invent for it after the fact. Without the repair the
//! migration would simply fail on deploy against any such row.
//!
//! `applied_at` is deliberately left alone by that repair. It records the date
//! an employer actually saw the application, which stays true whatever the
//! row's status now says, and the service preserves an existing stamp when an
//! application is sent again — so keeping it means the real date survives the
//! round trip rather than being replaced by the date of the re-send.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const CONSTRAINT: &str = "applications_sent_requires_snapshot";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Repair before constraining: a row that cannot satisfy the invariant
        // would otherwise block the migration entirely.
        db.execute_unprepared(
            r#"UPDATE applications
               SET status = 'draft'
               WHERE status <> 'draft' AND cv_snapshot_id IS NULL"#,
        )
        .await?;

        // Dropped first so this can be re-run against a database that already
        // carries the constraint. Postgres has no `ADD CONSTRAINT IF NOT
        // EXISTS`, and a migration that cannot be retried after a partial
        // failure is worse than one that is merely repeated.
        db.execute_unprepared(&format!(
            "ALTER TABLE applications DROP CONSTRAINT IF EXISTS {CONSTRAINT}"
        ))
        .await?;

        db.execute_unprepared(&format!(
            r#"ALTER TABLE applications
               ADD CONSTRAINT {CONSTRAINT}
               CHECK (status = 'draft' OR cv_snapshot_id IS NOT NULL)"#
        ))
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(&format!(
                "ALTER TABLE applications DROP CONSTRAINT IF EXISTS {CONSTRAINT}"
            ))
            .await?;

        Ok(())
    }
}
