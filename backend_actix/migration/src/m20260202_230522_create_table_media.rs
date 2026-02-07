//! # Media Table Migration
//!
//! ## Purpose
//! The `media` table serves as the **single source of truth** for all uploaded files
//! in the application. It stores metadata about files stored in Google Cloud Storage,
//! completely decoupled from where those files are used.
//!
//! ## Design Philosophy
//! - **Separation of Concerns**: This table only knows about the file itself, not where
//!   it's attached. The `media_attachments` table handles relationships.
//! - **Ownership for Access Control**: Every media belongs to a user, enabling permission
//!   checks and cleanup when users are deleted.
//! - **Soft Delete**: Allows recovery and supports GCS cleanup jobs that run async.
//!
//! ## Key Columns Explained
//!
//! ### Storage Columns
//! - `bucket_name`: GCS bucket where the file lives. Supports multi-bucket strategies
//!   (e.g., separate buckets for images vs videos, or regional buckets).
//! - `object_key`: The full path/key in GCS. Combined with bucket, forms the unique
//!   storage location. Example: `users/123/avatars/abc123.webp`
//!
//! ### File Metadata
//! - `original_filename`: Preserved for download headers and UI display. Users see
//!   their original filename, not our internal object_key.
//! - `mime_type`: Critical for serving correct Content-Type headers and determining
//!   processing pipelines (image optimization, video transcoding, etc.)
//! - `file_size_bytes`: For quota enforcement, UI display, and upload validation.
//!
//! ### Media-Specific Metadata (nullable)
//! - `width`/`height`: For images and videos. Enables aspect ratio calculations,
//!   responsive image srcsets, and layout reservations (preventing CLS).
//! - `duration_seconds`: For video/audio. Enables duration display and validation
//!   (e.g., "max 30 second videos for projects").
//!
//! ### Processing Status
//! - `status`: Tracks async processing pipeline state.
//!   - `pending`: Just uploaded, awaiting processing
//!   - `processing`: Currently being optimized/transcoded
//!   - `ready`: Available for use
//!   - `failed`: Processing failed (check logs)
//!
//! ## Indexes
//! - `idx_media_user_id`: Fast lookup of all media owned by a user
//! - `idx_media_status`: For processing job queries ("find all pending media")
//! - `idx_media_active`: Partial index for non-deleted media lookups
//! - Unique constraint on (bucket_name, object_key) prevents duplicate storage refs

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // =====================================================
        // Create enum type for media.status
        // =====================================================
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DO $$
                BEGIN
                    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'media_status') THEN
                        CREATE TYPE media_status AS ENUM ('pending', 'processing', 'ready', 'failed');
                    END IF;
                END$$;
                "#,
            )
            .await?;

        // =====================================================
        // Create media table
        // =====================================================
        manager
            .create_table(
                Table::create()
                    .table(Media::Table)
                    .if_not_exists()
                    // Primary key
                    .col(
                        ColumnDef::new(Media::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("gen_random_uuid()")),
                    )
                    // Ownership - every file belongs to a user
                    .col(ColumnDef::new(Media::UserId).uuid().not_null())
                    // =========================================
                    // Storage location (GCS details)
                    // =========================================
                    // `bucket_name`: The GCS bucket where the file is stored.
                    //   - Supports multi-bucket strategies (e.g., separate buckets per media type or region)
                    //   - Example: "blogport-cms-avatars", "blogport-cms-media", "blogport-cms-videos"
                    //
                    // `object_key`: The full path/key within the bucket (everything after the bucket name).
                    //   - GCS has no real folders - "/" in keys creates virtual directory structure
                    //   - Combined with bucket_name, forms the complete gsutil URI: gs://{bucket_name}/{object_key}
                    //
                    // Examples:
                    //   | gsutil URI                                              | bucket_name            | object_key                          |
                    //   |---------------------------------------------------------|------------------------|-------------------------------------|
                    //   | gs://blogport-cms-avatars/7309700.jpg                   | blogport-cms-avatars   | 7309700.jpg                         |
                    //   | gs://blogport-cms-avatars/users/123/avatar.jpg          | blogport-cms-avatars   | users/123/avatar.jpg                |
                    //   | gs://blogport-cms-media/projects/456/screenshots/01.png | blogport-cms-media     | projects/456/screenshots/01.png     |
                    //   | gs://blogport-cms-videos/resumes/789/intro.mp4          | blogport-cms-videos    | resumes/789/intro.mp4               |
                    .col(ColumnDef::new(Media::BucketName).string_len(255).not_null())
                    .col(ColumnDef::new(Media::ObjectKey).string_len(1024).not_null())
                    // =========================================
                    // File metadata
                    // =========================================
                    .col(
                        ColumnDef::new(Media::OriginalFilename)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Media::MimeType).string_len(127).not_null())
                    .col(
                        ColumnDef::new(Media::FileSizeBytes)
                            .big_integer()
                            .not_null(),
                    )
                    // =========================================
                    // Media-specific metadata (nullable)
                    // =========================================
                    // Dimensions for images/videos
                    .col(ColumnDef::new(Media::Width).integer())
                    .col(ColumnDef::new(Media::Height).integer())
                    // Duration for videos/audio (in seconds, with decimals)
                    .col(ColumnDef::new(Media::DurationSeconds).decimal_len(10, 2))
                    // =========================================
                    // Processing status
                    // =========================================
                    .col(
                        ColumnDef::new(Media::Status)
                            .custom(Alias::new("media_status"))
                            .not_null()
                            .default(Expr::cust("'pending'::media_status")),
                    )
                    // =========================================
                    // Metadata (JSON)
                    // =========================================
                    // Record processing
                    .col(
                        ColumnDef::new(Media::Metadata)
                            .json_binary()
                            .not_null()
                            .default(Expr::cust("'{}'::jsonb")),
                    )
                    // =========================================
                    // Audit timestamps
                    // =========================================
                    .col(
                        ColumnDef::new(Media::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Media::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    // Soft delete - allows async GCS cleanup
                    .col(ColumnDef::new(Media::DeletedAt).timestamp_with_time_zone())
                    // =========================================
                    // Foreign keys
                    // =========================================
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_media_user_id")
                            .from(Media::Table, Media::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // =====================================================
        // Indexes
        // =====================================================

        // Fast lookup by owner - essential for "my uploads" queries
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX idx_media_user_id
                ON media (user_id);
                "#,
            )
            .await?;

        // For processing job queries: find pending/failed media
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX idx_media_status
                ON media (status)
                WHERE deleted_at IS NULL;
                "#,
            )
            .await?;

        // Partial index for active (non-deleted) media
        // Speeds up most queries since we rarely query deleted media
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX idx_media_active
                ON media (user_id, created_at DESC)
                WHERE deleted_at IS NULL;
                "#,
            )
            .await?;

        // Enforce unique storage location
        // Prevents accidental duplicate references to same GCS object
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE UNIQUE INDEX idx_media_bucket_object_unique
                ON media (bucket_name, object_key);
                "#,
            )
            .await?;

        // For MIME type filtering (e.g., "show only images")
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX idx_media_mime_type
                ON media (mime_type)
                WHERE deleted_at IS NULL;
                "#,
            )
            .await?;

        // =====================================================
        // updated_at trigger
        // =====================================================
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TRIGGER update_media_updated_at
                BEFORE UPDATE ON media
                FOR EACH ROW
                EXECUTE FUNCTION update_updated_at_column();
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop trigger
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP TRIGGER IF EXISTS update_media_updated_at ON media;
                "#,
            )
            .await?;

        // Drop indexes
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP INDEX IF EXISTS idx_media_user_id;
                DROP INDEX IF EXISTS idx_media_status;
                DROP INDEX IF EXISTS idx_media_active;
                DROP INDEX IF EXISTS idx_media_bucket_object_unique;
                DROP INDEX IF EXISTS idx_media_mime_type;
                "#,
            )
            .await?;

        // Drop table
        manager
            .drop_table(Table::drop().table(Media::Table).to_owned())
            .await?;

        // Drop enum type (only if nothing else depends on it)
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP TYPE IF EXISTS media_status;
                "#,
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Media {
    Table,
    Id,
    UserId,
    BucketName,
    ObjectKey,
    OriginalFilename,
    MimeType,
    FileSizeBytes,
    Width,
    Height,
    DurationSeconds,
    Status,
    Metadata,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
