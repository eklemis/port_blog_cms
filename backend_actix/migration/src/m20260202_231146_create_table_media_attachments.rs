//! # Media Attachments Table Migration
//!
//! ## Purpose
//! The `media_attachments` table is a **polymorphic junction table** that links media
//! files to any entity in the system. This is the "flexibility" layer that allows
//! unlimited growth in media use cases without schema changes.
//!
//! ## Design Philosophy
//! - **Polymorphic Association**: Instead of separate `user_avatars`, `project_screenshots`,
//!   `resume_photos` tables, we have one table that can link to anything.
//! - **Role-Based Semantics**: The `role` column gives context to the relationship.
//!   A media file attached to a blog could be a "cover", "inline_image", or "thumbnail".
//! - **Reusability**: Same media can be attached to multiple entities (if business rules allow).
//! - **Ordering Support**: The `position` column enables ordered galleries/carousels.
//!
//! ## Key Columns Explained
//!
//! ### Polymorphic Reference
//! - `attachable_type`: The entity type (table name). Examples:
//!   - `"user"` - for user avatars
//!   - `"resume"` - for resume profile photos
//!   - `"project"` - for project screenshots
//!   - `"blog_post"` - for blog images (future)
//! - `attachable_id`: The UUID of the entity being attached to.
//!
//! ### Relationship Context
//! - `role`: Describes the purpose of this attachment. Examples:
//!   - `"avatar"` - user profile photo
//!   - `"photo"` - resume profile photo
//!   - `"screenshot"` - project screenshot
//!   - `"cover"` - blog cover image
//!   - `"inline"` - embedded in content
//!   - `"gallery"` - part of an image gallery
//! - `position`: For ordered collections. 0-indexed. Enables drag-and-drop reordering.
//!
//! ### Attachment Metadata
//! - `alt_text`: Accessibility text for images. Important for SEO and screen readers.
//! - `caption`: Optional description/caption shown in UI.
//!
//! ## Trade-offs
//! - **No FK to attachable_id**: We can't have a foreign key since it points to different
//!   tables based on `attachable_type`. Referential integrity is enforced at application level.
//! - **Type Safety**: The `attachable_type` is a string, not an enum, for flexibility.
//!   Application code should validate allowed types.
//!
//! ## Indexes
//! - `idx_media_attachments_attachable`: Primary lookup pattern - "get all media for entity X"
//! - `idx_media_attachments_media_id`: For cascade operations and "where is this media used?"
//! - Unique constraint prevents duplicate attachments (same media + entity + role)
//!
//! ## Usage Examples
//! ```sql
//! -- User avatar
//! INSERT INTO media_attachments (media_id, attachable_type, attachable_id, role)
//! VALUES ('media-uuid', 'user', 'user-uuid', 'avatar');
//!
//! -- Project with ordered screenshots
//! INSERT INTO media_attachments (media_id, attachable_type, attachable_id, role, position)
//! VALUES
//!     ('img1-uuid', 'project', 'proj-uuid', 'screenshot', 0),
//!     ('img2-uuid', 'project', 'proj-uuid', 'screenshot', 1),
//!     ('vid1-uuid', 'project', 'proj-uuid', 'screenshot', 2);
//!
//! -- Blog with cover and inline images
//! INSERT INTO media_attachments (media_id, attachable_type, attachable_id, role)
//! VALUES
//!     ('cover-uuid', 'blog_post', 'blog-uuid', 'cover'),
//!     ('inline-uuid', 'blog_post', 'blog-uuid', 'inline');
//! ```

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // =====================================================
        // Create media_attachments table
        // =====================================================
        manager
            .create_table(
                Table::create()
                    .table(MediaAttachments::Table)
                    .if_not_exists()
                    // Primary key
                    .col(
                        ColumnDef::new(MediaAttachments::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("gen_random_uuid()")),
                    )
                    // Reference to the media file
                    .col(ColumnDef::new(MediaAttachments::MediaId).uuid().not_null())
                    // =========================================
                    // Polymorphic reference
                    // =========================================
                    // The type of entity this media is attached to
                    // Examples: 'user', 'resume', 'project', 'blog_post'
                    .col(
                        ColumnDef::new(MediaAttachments::AttachableType)
                            .string_len(100)
                            .not_null(),
                    )
                    // The UUID of the entity
                    .col(
                        ColumnDef::new(MediaAttachments::AttachableId)
                            .uuid()
                            .not_null(),
                    )
                    // =========================================
                    // Relationship context
                    // =========================================
                    // The role/purpose of this attachment
                    // Examples: 'avatar', 'cover', 'screenshot', 'gallery', 'inline'
                    .col(
                        ColumnDef::new(MediaAttachments::Role)
                            .string_len(100)
                            .not_null(),
                    )
                    // For ordered collections (galleries, screenshots)
                    // 0-indexed, allows reordering
                    .col(
                        ColumnDef::new(MediaAttachments::Position)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    // =========================================
                    // Attachment metadata
                    // =========================================
                    // Alt text for accessibility and SEO
                    .col(ColumnDef::new(MediaAttachments::AltText).string_len(500))
                    // Optional caption/description
                    .col(ColumnDef::new(MediaAttachments::Caption).text())
                    // =========================================
                    // Audit timestamp
                    // =========================================
                    .col(
                        ColumnDef::new(MediaAttachments::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    // =========================================
                    // Foreign keys
                    // =========================================
                    // Only FK to media - attachable_id is polymorphic
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_media_attachments_media_id")
                            .from(MediaAttachments::Table, MediaAttachments::MediaId)
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

        // Primary lookup: "get all media for this entity"
        // This is the most common query pattern
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX idx_media_attachments_attachable
                ON media_attachments (attachable_type, attachable_id);
                "#,
            )
            .await?;

        // Reverse lookup: "where is this media used?"
        // Useful for cascade operations and usage tracking
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX idx_media_attachments_media_id
                ON media_attachments (media_id);
                "#,
            )
            .await?;

        // For ordered retrieval within a role
        // e.g., "get project screenshots in order"
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX idx_media_attachments_position
                ON media_attachments (attachable_type, attachable_id, role, position);
                "#,
            )
            .await?;

        // Prevent duplicate attachments
        // Same media can't be attached to same entity with same role twice
        // (but CAN be attached with different roles, e.g., as both 'thumbnail' and 'gallery')
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE UNIQUE INDEX idx_media_attachments_unique
                ON media_attachments (media_id, attachable_type, attachable_id, role);
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
                DROP INDEX IF EXISTS idx_media_attachments_attachable;
                DROP INDEX IF EXISTS idx_media_attachments_media_id;
                DROP INDEX IF EXISTS idx_media_attachments_position;
                DROP INDEX IF EXISTS idx_media_attachments_unique;
                "#,
            )
            .await?;

        // Drop table
        manager
            .drop_table(Table::drop().table(MediaAttachments::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum MediaAttachments {
    Table,
    Id,
    MediaId,
    AttachableType,
    AttachableId,
    Role,
    Position,
    AltText,
    Caption,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Media {
    Table,
    Id,
}
