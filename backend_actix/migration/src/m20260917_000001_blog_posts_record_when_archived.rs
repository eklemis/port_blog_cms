//! Records when a post was archived, instead of inferring it.
//!
//! The console's archive list shows an "Archived" column. There was no column
//! to read it from, so it showed `updated_at` — correct only because every
//! write path filters `is_deleted = false`, which makes an archived row
//! unwritable and freezes its `updated_at` at the moment it was archived.
//!
//! That is an invariant held up by every writer remembering a `WHERE` clause,
//! and `blog_posts` has a `BEFORE UPDATE` trigger that stamps `updated_at` on
//! any update at all. One future statement that forgets — a backfill, an admin
//! fix, a new bulk operation — moves the date silently, and a date that is
//! wrong without saying so is worse than a blank column.
//!
//! So the date becomes a fact: `deleted_at`, maintained by the database on the
//! transition itself rather than by each writer.
//!
//! **Why a trigger and not a service write.** Migrations apply while the old
//! build is still serving (ADR 0003, ADR 0010), so between this migration and
//! the deploy the running code archives posts knowing nothing about the new
//! column. A trigger covers those writes too. It also cannot be forgotten by
//! code written later, which is the failure this is here to prevent.
//!
//! No `CHECK (is_deleted = (deleted_at IS NOT NULL))`, deliberately: a
//! constraint the running build does not know about would reject its writes.
//! The trigger makes that invariant true; a later deploy can enforce it.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Nullable, no default: null means "not archived", which is exactly
        // what every existing unarchived row already says.
        db.execute_unprepared(
            r#"
            ALTER TABLE blog_posts
                ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;
            "#,
        )
        .await?;

        db.execute_unprepared(
            r#"
            CREATE OR REPLACE FUNCTION blog_posts_stamp_deleted_at()
            RETURNS TRIGGER AS $$
            BEGIN
                IF NEW.is_deleted AND NOT OLD.is_deleted THEN
                    NEW.deleted_at = CURRENT_TIMESTAMP;
                ELSIF NOT NEW.is_deleted THEN
                    -- Restoring clears it, so a post archived twice reports the
                    -- second time rather than the first.
                    NEW.deleted_at = NULL;
                END IF;
                RETURN NEW;
            END;
            $$ language 'plpgsql';
            "#,
        )
        .await?;

        db.execute_unprepared(
            r#"
            DROP TRIGGER IF EXISTS stamp_blog_posts_deleted_at ON blog_posts;
            CREATE TRIGGER stamp_blog_posts_deleted_at
            BEFORE UPDATE OF is_deleted ON blog_posts
            FOR EACH ROW
            EXECUTE FUNCTION blog_posts_stamp_deleted_at();
            "#,
        )
        .await?;

        // Posts archived before this migration. `updated_at` is the archive
        // date for exactly the reason described above, so it is the best
        // record that exists — and the last moment it can be trusted.
        //
        // The `updated_at` trigger has to be off for this. It fires on any
        // update and would stamp every archived row with today, destroying the
        // dates being copied. Disabling it is transactional and scoped to this
        // statement; the new trigger is not created above by accident — it is
        // `BEFORE UPDATE OF is_deleted`, and this write does not touch that
        // column, so it does not fire either.
        db.execute_unprepared(
            r#"
            ALTER TABLE blog_posts DISABLE TRIGGER update_blog_posts_updated_at;

            UPDATE blog_posts
               SET deleted_at = updated_at
             WHERE is_deleted = true
               AND deleted_at IS NULL;

            ALTER TABLE blog_posts ENABLE TRIGGER update_blog_posts_updated_at;
            "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP TRIGGER IF EXISTS stamp_blog_posts_deleted_at ON blog_posts;
                DROP FUNCTION IF EXISTS blog_posts_stamp_deleted_at();
                ALTER TABLE blog_posts DROP COLUMN IF EXISTS deleted_at;
                "#,
            )
            .await?;

        Ok(())
    }
}
