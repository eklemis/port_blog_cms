//! # Media Variants Table Migration
//!
//! ## Purpose
//! The `media_variants` table stores **processed versions** of original media files.
//! This includes thumbnails, different resolutions, format conversions (WebP, AVIF),
//! and video transcodes (720p, 1080p).
//!
//! ## Design Philosophy
//! - **Separation from Original**: The `media` table holds the original upload.
//!   This table holds derived/processed versions. Originals are never modified.
//! - **On-Demand Generation**: Variants can be generated lazily (on first request)
//!   or eagerly (immediately after upload), depending on your processing strategy.
//! - **CDN-Friendly**: Each variant has its own GCS path, enabling direct CDN serving
//!   of the appropriate variant without server-side logic.
//!
//! ## Key Columns Explained
//!
//! ### Variant Identity
//! - `media_id`: Reference to the original media this is derived from.
//! - `variant_type`: A semantic identifier for this variant. Examples:
//!   - Image variants: `"thumbnail"`, `"small"`, `"medium"`, `"large"`, `"webp"`, `"avif"`
//!   - Video variants: `"360p"`, `"720p"`, `"1080p"`, `"thumbnail"`, `"gif_preview"`
//!   - Combined: `"thumbnail_webp"`, `"medium_avif"`
//!
//! ### Storage Details
//! - `bucket_name` / `object_key`: Where this variant lives in GCS.
//!   Typically same bucket as original, different path.
//!   Example: `users/123/avatars/abc123_thumbnail.webp`
//!
//! ### Variant Metadata
//! - `mime_type`: May differ from original (e.g., PNG original → WebP variant)
//! - `file_size_bytes`: For bandwidth optimization decisions
//! - `width` / `height`: For responsive image srcsets and aspect ratio preservation
//!
//! ## Common Variant Strategies
//!
//! ### Images (Responsive)
//! | variant_type | Typical Size | Use Case |
//! |-------------|-------------|----------|
//! | thumbnail   | 150x150     | Lists, grids |
//! | small       | 320px wide  | Mobile |
//! | medium      | 768px wide  | Tablet |
//! | large       | 1200px wide | Desktop |
//! | webp        | Same dims   | Modern browsers |
//! | avif        | Same dims   | Cutting-edge browsers |
//!
//! ### Videos
//! | variant_type | Resolution | Use Case |
//! |-------------|-----------|----------|
//! | 360p        | 640x360   | Low bandwidth |
//! | 720p        | 1280x720  | Default |
//! | 1080p       | 1920x1080 | High quality |
//! | thumbnail   | 320x180   | Video poster |
//! | gif_preview | 320px     | Hover preview |
//!
//! ## Processing Flow
//! 1. User uploads file → creates `media` record with `status='pending'`
//! 2. Background job processes original:
//!    - Generates variants
//!    - Creates `media_variants` records
//!    - Updates `media.status='ready'`
//! 3. API returns variant URLs based on client needs (viewport, connection)
//!
//! ## Indexes
//! - `idx_media_variants_media_id`: Find all variants for a media
//! - Unique constraint on (media_id, variant_type) prevents duplicates

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // =====================================================
        // Create media_variants table
        // =====================================================
        manager
            .create_table(
                Table::create()
                    .table(MediaVariants::Table)
                    .if_not_exists()
                    // Primary key
                    .col(
                        ColumnDef::new(MediaVariants::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("gen_random_uuid()")),
                    )
                    // Reference to original media
                    .col(ColumnDef::new(MediaVariants::MediaId).uuid().not_null())
                    // =========================================
                    // Variant identification
                    // =========================================
                    // Semantic identifier for this variant
                    // Examples: 'thumbnail', 'medium', '720p', 'webp', 'thumbnail_webp'
                    .col(
                        ColumnDef::new(MediaVariants::VariantType)
                            .string_len(50)
                            .not_null(),
                    )
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
                    .col(
                        ColumnDef::new(MediaVariants::BucketName)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MediaVariants::ObjectKey)
                            .string_len(1024)
                            .not_null(),
                    )
                    // =========================================
                    // Variant metadata
                    // =========================================
                    // May differ from original (e.g., PNG → WebP)
                    .col(
                        ColumnDef::new(MediaVariants::MimeType)
                            .string_len(127)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MediaVariants::FileSizeBytes)
                            .big_integer()
                            .not_null(),
                    )
                    // Dimensions (may differ from original)
                    .col(ColumnDef::new(MediaVariants::Width).integer())
                    .col(ColumnDef::new(MediaVariants::Height).integer())
                    // =========================================
                    // Audit timestamp
                    // =========================================
                    .col(
                        ColumnDef::new(MediaVariants::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    // =========================================
                    // Foreign keys
                    // =========================================
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_media_variants_media_id")
                            .from(MediaVariants::Table, MediaVariants::MediaId)
                            .to(Media::Table, Media::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // =====================================================
        // Indexes
        // =====================================================

        // Primary lookup: "get all variants for this media"
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX idx_media_variants_media_id
                ON media_variants (media_id);
                "#,
            )
            .await?;

        // Enforce one variant per type per media
        // Can't have two 'thumbnail' variants for the same media
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE UNIQUE INDEX idx_media_variants_unique
                ON media_variants (media_id, variant_type);
                "#,
            )
            .await?;

        // For finding variants by type across all media
        // Useful for batch operations: "regenerate all thumbnails"
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX idx_media_variants_type
                ON media_variants (variant_type);
                "#,
            )
            .await?;

        // Unique storage location (same as media table)
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE UNIQUE INDEX idx_media_variants_bucket_object_unique
                ON media_variants (bucket_name, object_key);
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP INDEX IF EXISTS idx_media_variants_media_id;
                DROP INDEX IF EXISTS idx_media_variants_unique;
                DROP INDEX IF EXISTS idx_media_variants_type;
                DROP INDEX IF EXISTS idx_media_variants_bucket_object_unique;
                "#,
            )
            .await?;

        // Drop table
        manager
            .drop_table(Table::drop().table(MediaVariants::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum MediaVariants {
    Table,
    Id,
    MediaId,
    VariantType,
    BucketName,
    ObjectKey,
    MimeType,
    FileSizeBytes,
    Width,
    Height,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Media {
    Table,
    Id,
}
