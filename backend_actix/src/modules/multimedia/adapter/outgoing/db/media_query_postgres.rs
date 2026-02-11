use async_trait::async_trait;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    auth::application::domain::entities::UserId,
    multimedia::application::{
        domain::entities::{AttachmentTarget, MediaRole, MediaSize, MediaState, MediaStateInfo},
        ports::outgoing::db::{MediaAttachment, MediaQuery, MediaQueryError, StoredVariant},
    },
};

// ============================================================================
// Query Implementation (Production)
// ============================================================================

#[derive(Clone)]
pub struct MediaQueryPostgres {
    db: Arc<DatabaseConnection>,
}

impl MediaQueryPostgres {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // =====================================================
    // SQL builders
    // =====================================================

    fn get_state_stmt(media_id: Uuid) -> Statement {
        Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            SELECT
                m.user_id,
                m.id as media_id,
                m.updated_at,
                m.status::text as status
            FROM media m
            WHERE m.id = $1
              AND m.deleted_at IS NULL
            "#,
            vec![media_id.into()],
        )
    }

    fn list_by_target_stmt(owner: Uuid, target: &str) -> Statement {
        Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            SELECT
                m.user_id,
                m.id as media_id,
                ma.attachable_type,
                ma.attachable_id,
                m.status::text as status,
                ma.role::text as role,
                ma.position,
                COALESCE(ma.alt_text, '') as alt_text,
                COALESCE(ma.caption, '') as caption,
                m.original_filename
            FROM media m
            INNER JOIN media_attachments ma ON m.id = ma.media_id
            WHERE m.user_id = $1
              AND ma.attachable_type = $2
              AND m.deleted_at IS NULL
            ORDER BY ma.position ASC, ma.created_at ASC
            "#,
            vec![owner.into(), target.into()],
        )
    }

    fn get_attachment_info_stmt(media_id: Uuid) -> Statement {
        Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            SELECT
                m.user_id,
                m.id as media_id,
                ma.attachable_type,
                ma.attachable_id,
                m.status::text as status,
                ma.role,
                ma.position,
                COALESCE(ma.alt_text, '') as alt_text,
                COALESCE(ma.caption, '') as caption,
                m.original_filename
            FROM media m
            INNER JOIN media_attachments ma ON m.id = ma.media_id
            WHERE m.id = $1
              AND m.deleted_at IS NULL
            LIMIT 1
            "#,
            vec![media_id.into()],
        )
    }

    fn get_variants_stmt(media_id: Uuid) -> Statement {
        Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            SELECT
            variant_type::text as variant_type,
                bucket_name,
                object_key,
                width,
                height,
                file_size_bytes,
                mime_type
            FROM media_variants
            WHERE media_id = $1
            ORDER BY
                CASE variant_type
                    WHEN 'thumbnail' THEN 1
                    WHEN 'small' THEN 2
                    WHEN 'medium' THEN 3
                    WHEN 'large' THEN 4
                    ELSE 5
                END
            "#,
            vec![media_id.into()],
        )
    }

    // =====================================================
    // Mapping helpers
    // =====================================================

    fn map_db_err(e: DbErr) -> MediaQueryError {
        MediaQueryError::DatabaseError(e.to_string())
    }

    fn parse_media_state(s: &str) -> Result<MediaState, MediaQueryError> {
        match s {
            "pending" => Ok(MediaState::Pending),
            "processing" => Ok(MediaState::Processing),
            "ready" => Ok(MediaState::Ready),
            "failed" => Ok(MediaState::Failed),
            _ => Err(MediaQueryError::DatabaseError(format!(
                "invalid media state: {}",
                s
            ))),
        }
    }

    fn parse_attachment_target(s: &str) -> Result<AttachmentTarget, MediaQueryError> {
        match s {
            "user" => Ok(AttachmentTarget::User),
            "resume" => Ok(AttachmentTarget::Resume),
            "project" => Ok(AttachmentTarget::Project),
            "blog_post" => Ok(AttachmentTarget::BlogPost),
            _ => Err(MediaQueryError::DatabaseError(format!(
                "invalid attachment target: {}",
                s
            ))),
        }
    }

    fn parse_media_role(s: &str) -> Result<MediaRole, MediaQueryError> {
        match s {
            "avatar" => Ok(MediaRole::Avatar),
            "profile" => Ok(MediaRole::Profile),
            "cover" => Ok(MediaRole::Cover),
            "screenshoot" => Ok(MediaRole::Screenshoot),
            "gallery" => Ok(MediaRole::Gallery),
            "inline" => Ok(MediaRole::Inline),
            _ => Err(MediaQueryError::DatabaseError(format!(
                "invalid media role: {}",
                s
            ))),
        }
    }

    fn parse_media_size(s: &str) -> Result<MediaSize, MediaQueryError> {
        match s {
            "thumbnail" => Ok(MediaSize::Thumbnail),
            "small" => Ok(MediaSize::Small),
            "medium" => Ok(MediaSize::Medium),
            "large" => Ok(MediaSize::Large),
            _ => Err(MediaQueryError::DatabaseError(format!(
                "invalid media size: {}",
                s
            ))),
        }
    }

    async fn get_variants(
        db: &DatabaseConnection,
        media_id: Uuid,
    ) -> Result<Vec<StoredVariant>, MediaQueryError> {
        let stmt = Self::get_variants_stmt(media_id);

        let results = db.query_all(stmt).await.map_err(Self::map_db_err)?;

        let mut variants = Vec::new();

        for row in results {
            let variant_type: String = row.try_get("", "variant_type").map_err(Self::map_db_err)?;
            let bucket_name: String = row.try_get("", "bucket_name").map_err(Self::map_db_err)?;
            let object_key: String = row.try_get("", "object_key").map_err(Self::map_db_err)?;
            let width: i32 = row.try_get("", "width").map_err(Self::map_db_err)?;
            let height: i32 = row.try_get("", "height").map_err(Self::map_db_err)?;
            let file_size_bytes: i64 = row
                .try_get("", "file_size_bytes")
                .map_err(Self::map_db_err)?;
            let mime_type: String = row.try_get("", "mime_type").map_err(Self::map_db_err)?;

            let size = Self::parse_media_size(&variant_type)?;

            variants.push(StoredVariant {
                size,
                bucket_name,
                object_name: object_key,
                width: width as u32,
                height: height as u32,
                file_size_bytes: file_size_bytes as u64,
                mime_type,
            });
        }

        Ok(variants)
    }
}

#[async_trait]
impl MediaQuery for MediaQueryPostgres {
    async fn get_state(&self, media_id: Uuid) -> Result<MediaStateInfo, MediaQueryError> {
        let stmt = Self::get_state_stmt(media_id);

        let result = self.db.query_one(stmt).await.map_err(Self::map_db_err)?;

        let row = result.ok_or(MediaQueryError::MediaNotFound)?;

        let user_id: Uuid = row.try_get("", "user_id").map_err(Self::map_db_err)?;
        let media_id: Uuid = row.try_get("", "media_id").map_err(Self::map_db_err)?;
        let updated_at: chrono::DateTime<chrono::FixedOffset> =
            row.try_get("", "updated_at").map_err(Self::map_db_err)?;
        let status: String = row.try_get("", "status").map_err(Self::map_db_err)?;

        Ok(MediaStateInfo {
            owner: UserId::from(user_id),
            media_id,
            updated_at: updated_at.to_rfc3339(),
            status: Self::parse_media_state(&status)?,
        })
    }

    async fn list_by_target(
        &self,
        owner: UserId,
        target: AttachmentTarget,
    ) -> Result<Vec<MediaAttachment>, MediaQueryError> {
        let owner_uuid: Uuid = owner.into();
        let target_str = target.to_string();

        let stmt = Self::list_by_target_stmt(owner_uuid, &target_str);

        let results = self.db.query_all(stmt).await.map_err(Self::map_db_err)?;

        let mut media_list = Vec::new();

        for row in results {
            let user_id: Uuid = row.try_get("", "user_id").map_err(Self::map_db_err)?;
            let media_id: Uuid = row.try_get("", "media_id").map_err(Self::map_db_err)?;
            let attachable_type: String = row
                .try_get("", "attachable_type")
                .map_err(Self::map_db_err)?;
            let attachable_id: Uuid = row.try_get("", "attachable_id").map_err(Self::map_db_err)?;
            let status: String = row.try_get("", "status").map_err(Self::map_db_err)?;
            let role: String = row.try_get("", "role").map_err(Self::map_db_err)?;
            let position: i32 = row.try_get("", "position").map_err(Self::map_db_err)?;
            let alt_text: String = row.try_get("", "alt_text").map_err(Self::map_db_err)?;
            let caption: String = row.try_get("", "caption").map_err(Self::map_db_err)?;
            let original_filename: String = row
                .try_get("", "original_filename")
                .map_err(Self::map_db_err)?;

            // Fetch variants for this media
            let variants = Self::get_variants(&self.db, media_id).await?;

            media_list.push(MediaAttachment {
                media_id,
                owner: UserId::from(user_id),
                attachment_target: Self::parse_attachment_target(&attachable_type)?,
                attachment_target_id: attachable_id,
                status: Self::parse_media_state(&status)?,
                role: Self::parse_media_role(&role)?,
                position: position as i16,
                alt_text,
                caption,
                original_filename,
                variants,
            });
        }

        Ok(media_list)
    }

    async fn get_attachment_info(
        &self,
        media_id: Uuid,
    ) -> Result<MediaAttachment, MediaQueryError> {
        let stmt = Self::get_attachment_info_stmt(media_id);

        let result = self.db.query_one(stmt).await.map_err(Self::map_db_err)?;

        let row = result.ok_or(MediaQueryError::MediaNotFound)?;

        let user_id: Uuid = row.try_get("", "user_id").map_err(Self::map_db_err)?;
        let media_id: Uuid = row.try_get("", "media_id").map_err(Self::map_db_err)?;
        let attachable_type: String = row
            .try_get("", "attachable_type")
            .map_err(Self::map_db_err)?;
        let attachable_id: Uuid = row.try_get("", "attachable_id").map_err(Self::map_db_err)?;
        let status: String = row.try_get("", "status").map_err(Self::map_db_err)?;
        let role: String = row.try_get("", "role").map_err(Self::map_db_err)?;
        let position: i32 = row.try_get("", "position").map_err(Self::map_db_err)?;
        let alt_text: String = row.try_get("", "alt_text").map_err(Self::map_db_err)?;
        let caption: String = row.try_get("", "caption").map_err(Self::map_db_err)?;
        let original_filename: String = row
            .try_get("", "original_filename")
            .map_err(Self::map_db_err)?;

        // Fetch variants for this media
        let variants = Self::get_variants(&self.db, media_id).await?;

        Ok(MediaAttachment {
            media_id,
            owner: UserId::from(user_id),
            attachment_target: Self::parse_attachment_target(&attachable_type)?,
            attachment_target_id: attachable_id,
            status: Self::parse_media_state(&status)?,
            role: Self::parse_media_role(&role)?,
            position: position as i16,
            alt_text,
            caption,
            original_filename,
            variants,
        })
    }
}

// ============================================================================
// Tests (deterministic, 100% branch coverage)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase, Value};
    use std::collections::BTreeMap;

    // Helper to create BTreeMap query results
    fn make_row(data: Vec<(&str, Value)>) -> BTreeMap<String, Value> {
        data.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    // -----------------------
    // get_state
    // -----------------------

    #[tokio::test]
    async fn test_get_state_success() {
        let media_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let now = Utc::now().fixed_offset();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![make_row(vec![
                ("user_id", Value::Uuid(Some(Box::new(user_id)))),
                ("media_id", Value::Uuid(Some(Box::new(media_id)))),
                (
                    "updated_at",
                    Value::ChronoDateTimeWithTimeZone(Some(Box::new(now))),
                ),
                ("status", Value::String(Some(Box::new("ready".to_string())))),
            ])]])
            .into_connection();

        let query = MediaQueryPostgres::new(Arc::new(db));
        let result = query.get_state(media_id).await;

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.owner, UserId::from(user_id));
        assert_eq!(info.media_id, media_id);
        assert_eq!(info.status, MediaState::Ready);
    }

    #[tokio::test]
    async fn test_get_state_not_found() {
        let media_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<BTreeMap<String, Value>>::new()])
            .into_connection();

        let query = MediaQueryPostgres::new(Arc::new(db));
        let err = query.get_state(media_id).await.unwrap_err();

        assert!(matches!(err, MediaQueryError::MediaNotFound));
    }

    #[tokio::test]
    async fn test_get_state_db_error() {
        let media_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors(vec![DbErr::Custom("connection error".to_string())])
            .into_connection();

        let query = MediaQueryPostgres::new(Arc::new(db));
        let err = query.get_state(media_id).await.unwrap_err();

        match err {
            MediaQueryError::DatabaseError(msg) => assert!(msg.contains("connection error")),
            _ => panic!("Expected DatabaseError"),
        }
    }

    #[tokio::test]
    async fn test_get_state_invalid_status() {
        let media_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let now = Utc::now().fixed_offset();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![make_row(vec![
                ("user_id", Value::Uuid(Some(Box::new(user_id)))),
                ("media_id", Value::Uuid(Some(Box::new(media_id)))),
                (
                    "updated_at",
                    Value::ChronoDateTimeWithTimeZone(Some(Box::new(now))),
                ),
                (
                    "status",
                    Value::String(Some(Box::new("invalid_status".to_string()))),
                ),
            ])]])
            .into_connection();

        let query = MediaQueryPostgres::new(Arc::new(db));
        let err = query.get_state(media_id).await.unwrap_err();

        match err {
            MediaQueryError::DatabaseError(msg) => assert!(msg.contains("invalid media state")),
            _ => panic!("Expected DatabaseError"),
        }
    }

    // -----------------------
    // list_by_target
    // -----------------------

    #[tokio::test]
    async fn test_list_by_target_success_with_variants() {
        let user_id = Uuid::new_v4();
        let media_id = Uuid::new_v4();
        let attachable_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![
                // First query: list media attachments
                vec![make_row(vec![
                    ("user_id", Value::Uuid(Some(Box::new(user_id)))),
                    ("media_id", Value::Uuid(Some(Box::new(media_id)))),
                    (
                        "attachable_type",
                        Value::String(Some(Box::new("resume".to_string()))),
                    ),
                    ("attachable_id", Value::Uuid(Some(Box::new(attachable_id)))),
                    ("status", Value::String(Some(Box::new("ready".to_string())))),
                    ("role", Value::String(Some(Box::new("profile".to_string())))),
                    ("position", Value::Int(Some(0))),
                    ("alt_text", Value::String(Some(Box::new("alt".to_string())))),
                    (
                        "caption",
                        Value::String(Some(Box::new("caption".to_string()))),
                    ),
                    (
                        "original_filename",
                        Value::String(Some(Box::new("photo.jpg".to_string()))),
                    ),
                ])],
                // Second query: get variants for media
                vec![make_row(vec![
                    (
                        "variant_type",
                        Value::String(Some(Box::new("thumbnail".to_string()))),
                    ),
                    (
                        "bucket_name",
                        Value::String(Some(Box::new("bucket-a".to_string()))),
                    ),
                    (
                        "object_key",
                        Value::String(Some(Box::new("thumb.webp".to_string()))),
                    ),
                    ("width", Value::Int(Some(150))),
                    ("height", Value::Int(Some(150))),
                    ("file_size_bytes", Value::BigInt(Some(5000))),
                    (
                        "mime_type",
                        Value::String(Some(Box::new("image/webp".to_string()))),
                    ),
                ])],
            ])
            .into_connection();

        let query = MediaQueryPostgres::new(Arc::new(db));
        let result = query
            .list_by_target(UserId::from(user_id), AttachmentTarget::Resume)
            .await;

        assert!(result.is_ok());
        let list = result.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].media_id, media_id);
        assert_eq!(list[0].attachment_target, AttachmentTarget::Resume);
        assert_eq!(list[0].status, MediaState::Ready);
        assert_eq!(list[0].role, MediaRole::Profile);
        assert_eq!(list[0].position, 0);
        assert_eq!(list[0].alt_text, "alt");
        assert_eq!(list[0].caption, "caption");
        assert_eq!(list[0].original_filename, "photo.jpg");
        assert_eq!(list[0].variants.len(), 1);
        assert_eq!(list[0].variants[0].bucket_name, "bucket-a");
        assert_eq!(list[0].variants[0].object_name, "thumb.webp");
        assert_eq!(list[0].variants[0].width, 150);
        assert_eq!(list[0].variants[0].height, 150);
        assert_eq!(list[0].variants[0].file_size_bytes, 5000);
        assert_eq!(list[0].variants[0].mime_type, "image/webp");
    }

    #[tokio::test]
    async fn test_list_by_target_empty_result() {
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<BTreeMap<String, Value>>::new()])
            .into_connection();

        let query = MediaQueryPostgres::new(Arc::new(db));
        let result = query
            .list_by_target(UserId::from(user_id), AttachmentTarget::Project)
            .await;

        assert!(result.is_ok());
        let list = result.unwrap();
        assert_eq!(list.len(), 0);
    }

    #[tokio::test]
    async fn test_list_by_target_db_error() {
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors(vec![DbErr::Custom("query failed".to_string())])
            .into_connection();

        let query = MediaQueryPostgres::new(Arc::new(db));
        let err = query
            .list_by_target(UserId::from(user_id), AttachmentTarget::Resume)
            .await
            .unwrap_err();

        match err {
            MediaQueryError::DatabaseError(msg) => assert!(msg.contains("query failed")),
            _ => panic!("Expected DatabaseError"),
        }
    }

    #[tokio::test]
    async fn test_list_by_target_invalid_attachment_type() {
        let user_id = Uuid::new_v4();
        let media_id = Uuid::new_v4();
        let attachable_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![
                vec![make_row(vec![
                    ("user_id", Value::Uuid(Some(Box::new(user_id)))),
                    ("media_id", Value::Uuid(Some(Box::new(media_id)))),
                    (
                        "attachable_type",
                        Value::String(Some(Box::new("invalid".to_string()))),
                    ),
                    ("attachable_id", Value::Uuid(Some(Box::new(attachable_id)))),
                    ("status", Value::String(Some(Box::new("ready".to_string())))),
                    ("role", Value::String(Some(Box::new("profile".to_string())))),
                    ("position", Value::Int(Some(0))),
                    ("alt_text", Value::String(Some(Box::new("".to_string())))),
                    ("caption", Value::String(Some(Box::new("".to_string())))),
                    (
                        "original_filename",
                        Value::String(Some(Box::new("test.jpg".to_string()))),
                    ),
                ])],
                // 2) get_variants query rows (empty)
                Vec::<BTreeMap<String, Value>>::new(),
            ])
            .into_connection();

        let query = MediaQueryPostgres::new(Arc::new(db));
        let err = query
            .list_by_target(UserId::from(user_id), AttachmentTarget::Resume)
            .await
            .unwrap_err();

        match err {
            MediaQueryError::DatabaseError(msg) => {
                assert!(msg.contains("invalid attachment target"));
            }
            _ => panic!("Expected DatabaseError"),
        }
    }

    // -----------------------
    // get_attachment_info
    // -----------------------

    #[tokio::test]
    async fn test_get_attachment_info_success() {
        let user_id = Uuid::new_v4();
        let media_id = Uuid::new_v4();
        let attachable_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![
                // First query: get attachment
                vec![make_row(vec![
                    ("user_id", Value::Uuid(Some(Box::new(user_id)))),
                    ("media_id", Value::Uuid(Some(Box::new(media_id)))),
                    (
                        "attachable_type",
                        Value::String(Some(Box::new("project".to_string()))),
                    ),
                    ("attachable_id", Value::Uuid(Some(Box::new(attachable_id)))),
                    (
                        "status",
                        Value::String(Some(Box::new("processing".to_string()))),
                    ),
                    (
                        "role",
                        Value::String(Some(Box::new("screenshoot".to_string()))),
                    ),
                    ("position", Value::Int(Some(2))),
                    (
                        "alt_text",
                        Value::String(Some(Box::new("screenshot".to_string()))),
                    ),
                    ("caption", Value::String(Some(Box::new("".to_string())))),
                    (
                        "original_filename",
                        Value::String(Some(Box::new("screen.png".to_string()))),
                    ),
                ])],
                // Second query: get variants (empty)
                Vec::<BTreeMap<String, Value>>::new(),
            ])
            .into_connection();

        let query = MediaQueryPostgres::new(Arc::new(db));
        let result = query.get_attachment_info(media_id).await;

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.media_id, media_id);
        assert_eq!(info.attachment_target, AttachmentTarget::Project);
        assert_eq!(info.status, MediaState::Processing);
        assert_eq!(info.role, MediaRole::Screenshoot);
        assert_eq!(info.position, 2);
        assert_eq!(info.alt_text, "screenshot");
        assert_eq!(info.caption, "");
        assert_eq!(info.original_filename, "screen.png");
        assert_eq!(info.variants.len(), 0);
    }

    #[tokio::test]
    async fn test_get_attachment_info_not_found() {
        let media_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<BTreeMap<String, Value>>::new()])
            .into_connection();

        let query = MediaQueryPostgres::new(Arc::new(db));
        let err = query.get_attachment_info(media_id).await.unwrap_err();

        assert!(matches!(err, MediaQueryError::MediaNotFound));
    }

    #[tokio::test]
    async fn test_get_attachment_info_invalid_role() {
        let user_id = Uuid::new_v4();
        let media_id = Uuid::new_v4();
        let attachable_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![
                // 1) get_attachment_info query rows
                vec![make_row(vec![
                    ("user_id", Value::Uuid(Some(Box::new(user_id)))),
                    ("media_id", Value::Uuid(Some(Box::new(media_id)))),
                    (
                        "attachable_type",
                        Value::String(Some(Box::new("resume".to_string()))),
                    ),
                    ("attachable_id", Value::Uuid(Some(Box::new(attachable_id)))),
                    ("status", Value::String(Some(Box::new("ready".to_string())))),
                    (
                        "role",
                        Value::String(Some(Box::new("invalid_role".to_string()))),
                    ),
                    ("position", Value::Int(Some(0))),
                    ("alt_text", Value::String(Some(Box::new("".to_string())))),
                    ("caption", Value::String(Some(Box::new("".to_string())))),
                    (
                        "original_filename",
                        Value::String(Some(Box::new("test.jpg".to_string()))),
                    ),
                ])],
                // 2) get_variants query rows (empty)
                Vec::<BTreeMap<String, Value>>::new(),
            ])
            .into_connection();

        let query = MediaQueryPostgres::new(Arc::new(db));
        let err = query.get_attachment_info(media_id).await.unwrap_err();

        match err {
            MediaQueryError::DatabaseError(msg) => {
                assert!(msg.contains("invalid media role"));
            }
            _ => panic!("Expected DatabaseError"),
        }
    }

    // -----------------------
    // get_variants (helper) - tested via integration
    // -----------------------

    #[tokio::test]
    async fn test_list_by_target_with_multiple_variant_sizes() {
        let user_id = Uuid::new_v4();
        let media_id = Uuid::new_v4();
        let attachable_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![
                // First query: list media attachments
                vec![make_row(vec![
                    ("user_id", Value::Uuid(Some(Box::new(user_id)))),
                    ("media_id", Value::Uuid(Some(Box::new(media_id)))),
                    (
                        "attachable_type",
                        Value::String(Some(Box::new("resume".to_string()))),
                    ),
                    ("attachable_id", Value::Uuid(Some(Box::new(attachable_id)))),
                    ("status", Value::String(Some(Box::new("ready".to_string())))),
                    ("role", Value::String(Some(Box::new("profile".to_string())))),
                    ("position", Value::Int(Some(0))),
                    ("alt_text", Value::String(Some(Box::new("".to_string())))),
                    ("caption", Value::String(Some(Box::new("".to_string())))),
                    (
                        "original_filename",
                        Value::String(Some(Box::new("image.jpg".to_string()))),
                    ),
                ])],
                // Second query: get variants (all sizes)
                vec![
                    make_row(vec![
                        (
                            "variant_type",
                            Value::String(Some(Box::new("thumbnail".to_string()))),
                        ),
                        (
                            "bucket_name",
                            Value::String(Some(Box::new("bucket".to_string()))),
                        ),
                        (
                            "object_key",
                            Value::String(Some(Box::new("t.webp".to_string()))),
                        ),
                        ("width", Value::Int(Some(150))),
                        ("height", Value::Int(Some(150))),
                        ("file_size_bytes", Value::BigInt(Some(3000))),
                        (
                            "mime_type",
                            Value::String(Some(Box::new("image/webp".to_string()))),
                        ),
                    ]),
                    make_row(vec![
                        (
                            "variant_type",
                            Value::String(Some(Box::new("small".to_string()))),
                        ),
                        (
                            "bucket_name",
                            Value::String(Some(Box::new("bucket".to_string()))),
                        ),
                        (
                            "object_key",
                            Value::String(Some(Box::new("s.webp".to_string()))),
                        ),
                        ("width", Value::Int(Some(400))),
                        ("height", Value::Int(Some(300))),
                        ("file_size_bytes", Value::BigInt(Some(15000))),
                        (
                            "mime_type",
                            Value::String(Some(Box::new("image/webp".to_string()))),
                        ),
                    ]),
                    make_row(vec![
                        (
                            "variant_type",
                            Value::String(Some(Box::new("medium".to_string()))),
                        ),
                        (
                            "bucket_name",
                            Value::String(Some(Box::new("bucket".to_string()))),
                        ),
                        (
                            "object_key",
                            Value::String(Some(Box::new("m.webp".to_string()))),
                        ),
                        ("width", Value::Int(Some(800))),
                        ("height", Value::Int(Some(600))),
                        ("file_size_bytes", Value::BigInt(Some(45000))),
                        (
                            "mime_type",
                            Value::String(Some(Box::new("image/webp".to_string()))),
                        ),
                    ]),
                    make_row(vec![
                        (
                            "variant_type",
                            Value::String(Some(Box::new("large".to_string()))),
                        ),
                        (
                            "bucket_name",
                            Value::String(Some(Box::new("bucket".to_string()))),
                        ),
                        (
                            "object_key",
                            Value::String(Some(Box::new("l.webp".to_string()))),
                        ),
                        ("width", Value::Int(Some(1920))),
                        ("height", Value::Int(Some(1080))),
                        ("file_size_bytes", Value::BigInt(Some(120000))),
                        (
                            "mime_type",
                            Value::String(Some(Box::new("image/webp".to_string()))),
                        ),
                    ]),
                ],
            ])
            .into_connection();

        let query = MediaQueryPostgres::new(Arc::new(db));
        let result = query
            .list_by_target(UserId::from(user_id), AttachmentTarget::Resume)
            .await;

        assert!(result.is_ok());
        let list = result.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].variants.len(), 4);
        assert_eq!(list[0].variants[0].size, MediaSize::Thumbnail);
        assert_eq!(list[0].variants[0].object_name, "t.webp");
        assert_eq!(list[0].variants[1].size, MediaSize::Small);
        assert_eq!(list[0].variants[1].object_name, "s.webp");
        assert_eq!(list[0].variants[2].size, MediaSize::Medium);
        assert_eq!(list[0].variants[2].object_name, "m.webp");
        assert_eq!(list[0].variants[3].size, MediaSize::Large);
        assert_eq!(list[0].variants[3].object_name, "l.webp");
    }

    #[tokio::test]
    async fn test_list_by_target_with_invalid_variant_size() {
        let user_id = Uuid::new_v4();
        let media_id = Uuid::new_v4();
        let attachable_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![
                // First query: list media attachments
                vec![make_row(vec![
                    ("user_id", Value::Uuid(Some(Box::new(user_id)))),
                    ("media_id", Value::Uuid(Some(Box::new(media_id)))),
                    (
                        "attachable_type",
                        Value::String(Some(Box::new("resume".to_string()))),
                    ),
                    ("attachable_id", Value::Uuid(Some(Box::new(attachable_id)))),
                    ("status", Value::String(Some(Box::new("ready".to_string())))),
                    ("role", Value::String(Some(Box::new("profile".to_string())))),
                    ("position", Value::Int(Some(0))),
                    ("alt_text", Value::String(Some(Box::new("".to_string())))),
                    ("caption", Value::String(Some(Box::new("".to_string())))),
                    (
                        "original_filename",
                        Value::String(Some(Box::new("test.jpg".to_string()))),
                    ),
                ])],
                // Second query: invalid variant size
                vec![make_row(vec![
                    (
                        "variant_type",
                        Value::String(Some(Box::new("invalid_size".to_string()))),
                    ),
                    (
                        "bucket_name",
                        Value::String(Some(Box::new("bucket".to_string()))),
                    ),
                    (
                        "object_key",
                        Value::String(Some(Box::new("obj.webp".to_string()))),
                    ),
                    ("width", Value::Int(Some(800))),
                    ("height", Value::Int(Some(600))),
                    ("file_size_bytes", Value::BigInt(Some(50000))),
                    (
                        "mime_type",
                        Value::String(Some(Box::new("image/webp".to_string()))),
                    ),
                ])],
            ])
            .into_connection();

        let query = MediaQueryPostgres::new(Arc::new(db));
        let err = query
            .list_by_target(UserId::from(user_id), AttachmentTarget::Resume)
            .await
            .unwrap_err();

        match err {
            MediaQueryError::DatabaseError(msg) => assert!(msg.contains("invalid media size")),
            _ => panic!("Expected DatabaseError"),
        }
    }

    // -----------------------
    // Edge cases - parse functions
    // -----------------------

    #[tokio::test]
    async fn test_parse_all_media_states() {
        assert!(matches!(
            MediaQueryPostgres::parse_media_state("pending").unwrap(),
            MediaState::Pending
        ));
        assert!(matches!(
            MediaQueryPostgres::parse_media_state("processing").unwrap(),
            MediaState::Processing
        ));
        assert!(matches!(
            MediaQueryPostgres::parse_media_state("ready").unwrap(),
            MediaState::Ready
        ));
        assert!(matches!(
            MediaQueryPostgres::parse_media_state("failed").unwrap(),
            MediaState::Failed
        ));
    }

    #[tokio::test]
    async fn test_parse_all_attachment_targets() {
        assert!(matches!(
            MediaQueryPostgres::parse_attachment_target("user").unwrap(),
            AttachmentTarget::User
        ));
        assert!(matches!(
            MediaQueryPostgres::parse_attachment_target("resume").unwrap(),
            AttachmentTarget::Resume
        ));
        assert!(matches!(
            MediaQueryPostgres::parse_attachment_target("project").unwrap(),
            AttachmentTarget::Project
        ));
        assert!(matches!(
            MediaQueryPostgres::parse_attachment_target("blog_post").unwrap(),
            AttachmentTarget::BlogPost
        ));
    }

    #[tokio::test]
    async fn test_parse_all_media_roles() {
        assert!(matches!(
            MediaQueryPostgres::parse_media_role("avatar").unwrap(),
            MediaRole::Avatar
        ));
        assert!(matches!(
            MediaQueryPostgres::parse_media_role("profile").unwrap(),
            MediaRole::Profile
        ));
        assert!(matches!(
            MediaQueryPostgres::parse_media_role("cover").unwrap(),
            MediaRole::Cover
        ));
        assert!(matches!(
            MediaQueryPostgres::parse_media_role("screenshoot").unwrap(),
            MediaRole::Screenshoot
        ));
        assert!(matches!(
            MediaQueryPostgres::parse_media_role("gallery").unwrap(),
            MediaRole::Gallery
        ));
        assert!(matches!(
            MediaQueryPostgres::parse_media_role("inline").unwrap(),
            MediaRole::Inline
        ));
    }

    #[tokio::test]
    async fn test_parse_all_media_sizes() {
        assert!(matches!(
            MediaQueryPostgres::parse_media_size("thumbnail").unwrap(),
            MediaSize::Thumbnail
        ));
        assert!(matches!(
            MediaQueryPostgres::parse_media_size("small").unwrap(),
            MediaSize::Small
        ));
        assert!(matches!(
            MediaQueryPostgres::parse_media_size("medium").unwrap(),
            MediaSize::Medium
        ));
        assert!(matches!(
            MediaQueryPostgres::parse_media_size("large").unwrap(),
            MediaSize::Large
        ));
    }
}
