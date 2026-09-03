//! The Career Studio's spine: jobs, applications, and frozen CV snapshots.
//!
//! # Why a snapshot table rather than a version column on `resumes`
//!
//! An application points at the CV that was sent. If it pointed at the living
//! CV row, editing that CV would retroactively change what every past
//! application claims to have used — and the one question this feature exists
//! to answer, *what exactly did they read*, becomes unanswerable. The snapshot
//! is a separate, immutable artifact: written once when an application leaves
//! draft, never updated.
//!
//! # Why `no_reply` is a status and not an absence
//!
//! Silence is the most common outcome and the most informative one. A rejection
//! after three interviews and a posting that never replied are different events
//! with different lessons; folding them into one destroys the only pattern
//! worth surfacing later. The status is therefore explicit, and distinct from
//! `rejected`.
//!
//! # Why `source_text` is kept verbatim
//!
//! Postings get taken down. At interview time it is the only record of what was
//! actually asked for, so the original text is stored alongside whatever was
//! extracted from it — the extraction is derived data and may be re-derived,
//! the text cannot be recovered.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // ── CV snapshots ────────────────────────────────────────────────
        //
        // No `updated_at`, deliberately: there is no update path, and a column
        // that can never change is an invitation to write one.
        db.execute_unprepared(
            r#"
            CREATE TABLE IF NOT EXISTS cv_snapshots (
                id         UUID PRIMARY KEY,
                cv_id      UUID        NOT NULL REFERENCES resumes(id) ON DELETE CASCADE,
                user_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                -- The whole CV as it stood, not a diff. A diff would need the
                -- original to reconstruct, which is the thing that changes.
                document   JSONB       NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now()
            );
            "#,
        )
        .await?;

        db.execute_unprepared(
            r#"
            CREATE INDEX IF NOT EXISTS idx_cv_snapshots_cv
                ON cv_snapshots (cv_id, created_at DESC);
            "#,
        )
        .await?;

        // ── Jobs ────────────────────────────────────────────────────────
        db.execute_unprepared(
            r#"
            CREATE TABLE IF NOT EXISTS jobs (
                id              UUID PRIMARY KEY,
                user_id         UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                title           TEXT        NOT NULL,
                company         TEXT        NOT NULL,
                location        TEXT        NOT NULL DEFAULT '',
                seniority       TEXT        NOT NULL DEFAULT '',
                -- Extracted, and therefore re-derivable. Arrays of strings.
                required_skills JSONB       NOT NULL DEFAULT '[]'::jsonb,
                nice_to_have    JSONB       NOT NULL DEFAULT '[]'::jsonb,
                source_url      TEXT        NOT NULL DEFAULT '',
                -- Not re-derivable. See the module docs.
                source_text     TEXT        NOT NULL DEFAULT '',
                is_deleted      BOOLEAN     NOT NULL DEFAULT false,
                created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
                updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
            );
            "#,
        )
        .await?;

        db.execute_unprepared(
            r#"
            CREATE INDEX IF NOT EXISTS idx_jobs_owner
                ON jobs (user_id, created_at DESC) WHERE is_deleted = false;
            "#,
        )
        .await?;

        // ── Applications ────────────────────────────────────────────────
        //
        // `status` is a CHECK rather than a Postgres enum: adding a value to an
        // enum type is a migration that cannot run inside a transaction on
        // older Postgres, and the set will grow.
        db.execute_unprepared(
            r#"
            CREATE TABLE IF NOT EXISTS applications (
                id              UUID PRIMARY KEY,
                user_id         UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                job_id          UUID        NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
                -- Null while the application is a draft; set when it leaves.
                -- ON DELETE RESTRICT: a snapshot an application points at must
                -- not vanish, or the row starts lying again.
                cv_snapshot_id  UUID        REFERENCES cv_snapshots(id) ON DELETE RESTRICT,
                status          TEXT        NOT NULL DEFAULT 'draft',
                applied_at      TIMESTAMPTZ,
                next_action     TEXT        NOT NULL DEFAULT '',
                next_action_at  TIMESTAMPTZ,
                is_deleted      BOOLEAN     NOT NULL DEFAULT false,
                created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
                updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
                CONSTRAINT applications_status_check CHECK (status IN (
                    'draft', 'applied', 'screening', 'interview', 'final',
                    'offer', 'accepted', 'rejected', 'withdrawn', 'no_reply'
                ))
            );
            "#,
        )
        .await?;

        db.execute_unprepared(
            r#"
            CREATE INDEX IF NOT EXISTS idx_applications_owner
                ON applications (user_id, created_at DESC) WHERE is_deleted = false;
            "#,
        )
        .await?;

        // The tracker's other main view: what you owe something next.
        db.execute_unprepared(
            r#"
            CREATE INDEX IF NOT EXISTS idx_applications_next_action
                ON applications (user_id, next_action_at)
             WHERE is_deleted = false AND next_action_at IS NOT NULL;
            "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // Applications reference both of the others, so it goes first.
        db.execute_unprepared("DROP TABLE IF EXISTS applications;")
            .await?;
        db.execute_unprepared("DROP TABLE IF EXISTS jobs;").await?;
        db.execute_unprepared("DROP TABLE IF EXISTS cv_snapshots;")
            .await?;
        Ok(())
    }
}
