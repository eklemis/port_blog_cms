//! One shareable preview link per draft post.
//!
//! A draft is visible only to its author, so showing work in progress to a
//! reviewer without an account had no answer. This table backs that link.
//!
//! # Why a table rather than a signed token
//!
//! Password reset uses a stateless JWT, which needs no storage. That will not
//! do here: the author has to be able to **revoke** a link they have shared and
//! **see** which of their drafts are currently shared and until when. Neither
//! is possible with a token the server does not remember.
//!
//! # Why the token is stored as-is
//!
//! Reset tokens and passwords are hashed because the database must not be
//! enough to impersonate someone. That reasoning does not transfer. This token
//! authorises reading exactly one draft — and anyone reading this table can
//! already read `blog_posts`, so hashing would protect nothing it does not
//! already have. Storing it plainly is what lets the author's sharing panel
//! show the link again later instead of re-minting one and silently breaking
//! the reviewer's bookmark.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS blog_post_previews (
                    -- One link per post: renewing extends this row rather than
                    -- minting a second token, so a bookmark keeps working.
                    post_id    UUID PRIMARY KEY
                               REFERENCES blog_posts(id) ON DELETE CASCADE,
                    token      TEXT        NOT NULL UNIQUE,
                    expires_at TIMESTAMPTZ NOT NULL,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
                );
                "#,
            )
            .await?;

        // The public read is by token and nothing else, so it needs its own
        // index; the UNIQUE constraint above already provides one, and this
        // documents the access path rather than adding a second.
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX IF NOT EXISTS idx_blog_post_previews_expires_at
                    ON blog_post_previews (expires_at);
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(r#"DROP TABLE IF EXISTS blog_post_previews;"#)
            .await?;

        Ok(())
    }
}
