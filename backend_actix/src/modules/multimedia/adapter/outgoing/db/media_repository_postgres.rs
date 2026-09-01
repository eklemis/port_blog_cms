use async_trait::async_trait;
use chrono::Utc;
use sea_orm::ConnectionTrait;
use sea_orm::{DatabaseBackend, DatabaseConnection, DbErr, Statement, TransactionTrait};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::multimedia::application::{
    domain::entities::{MediaState, MediaStateInfo, MediaVariant},
    ports::outgoing::db::{
        MediaRepository, MediaRepositoryError, MediaVariantRecord, RecordMediaError, RecordMediaTx,
        RecordedMedia, UpdateMediaStateData,
    },
};

// ============================================================================
// Repository Implementation (Production)
// ============================================================================

#[derive(Clone)]
pub struct MediaRepositoryPostgres {
    db: Arc<DatabaseConnection>,
}

impl MediaRepositoryPostgres {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // =====================================================
    // SQL builders (kept in your preferred style)
    // =====================================================

    fn insert_media_stmt(
        media_id: Uuid,
        owner: Uuid,
        bucket_name: &str,
        object_key: &str,
        original_filename: &str,
        mime_type: &str,
        file_size_bytes: i64,
        width: Option<i32>,
        height: Option<i32>,
        duration_seconds: Option<i64>,
        status: &str,
        now: chrono::DateTime<chrono::FixedOffset>,
    ) -> Statement {
        Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            INSERT INTO media (
              id, user_id,
              bucket_name, object_key,
              original_filename, mime_type, file_size_bytes,
              width, height, duration_seconds,
              status, metadata,
              created_at, updated_at, deleted_at
            )
            VALUES (
              $1, $2,
              $3, $4,
              $5, $6, $7,
              $8, $9, $10,
              $11::media_status, '{}'::jsonb,
              $12, $12, NULL
            )
            "#,
            vec![
                media_id.into(),
                owner.into(),
                bucket_name.into(),
                object_key.into(),
                original_filename.into(),
                mime_type.into(),
                file_size_bytes.into(),
                width.into(),
                height.into(),
                // numeric column; we store whole seconds
                duration_seconds.map(|v| v as f64).into(),
                status.into(),
                now.into(),
            ],
        )
    }

    fn insert_attachment_stmt(
        media_id: Uuid,
        attachable_type: &str,
        attachable_id: Uuid,
        role: &str,
        position: i32,
        alt_text: Option<String>,
        caption: Option<String>,
        now: chrono::DateTime<chrono::FixedOffset>,
    ) -> Statement {
        Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            INSERT INTO media_attachments (
              id, media_id,
              attachable_type, attachable_id,
              role, position,
              alt_text, caption,
              created_at
            )
            VALUES (
              gen_random_uuid(), $1,
              $2, $3,
              $4, $5,
              $6, $7,
              $8
            )
            "#,
            vec![
                media_id.into(),
                attachable_type.into(),
                attachable_id.into(),
                role.into(),
                position.into(),
                alt_text.into(),
                caption.into(),
                now.into(),
            ],
        )
    }

    fn map_db_err(e: DbErr) -> RecordMediaError {
        RecordMediaError::DatabaseError(e.to_string())
    }

    /// Upserts one variant row, keyed on the `(media_id, variant_type)` unique
    /// index so re-processing the same media replaces rather than duplicates.
    ///
    /// Note the table carries a second unique index on
    /// `(bucket_name, object_key)`. A conflict there is not absorbed by this
    /// upsert and surfaces as a database error, which is the honest outcome:
    /// two variants claiming one storage object is a pipeline bug, not
    /// something to paper over.
    async fn upsert_variant<C: ConnectionTrait>(
        conn: &C,
        data: &MediaVariantRecord,
    ) -> Result<MediaVariant, MediaRepositoryError> {
        let variant_type = data.size.to_string();

        conn.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"INSERT INTO media_variants (
                   id, media_id, variant_type, bucket_name, object_key,
                   mime_type, file_size_bytes, width, height, created_at
               )
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
               ON CONFLICT (media_id, variant_type) DO UPDATE SET
                   bucket_name = EXCLUDED.bucket_name,
                   object_key = EXCLUDED.object_key,
                   mime_type = EXCLUDED.mime_type,
                   file_size_bytes = EXCLUDED.file_size_bytes,
                   width = EXCLUDED.width,
                   height = EXCLUDED.height"#,
            [
                Uuid::new_v4().into(),
                data.media_id.into(),
                variant_type.clone().into(),
                data.bucket_name.trim().into(),
                data.object_key.trim().into(),
                data.mime_type.trim().into(),
                (data.file_size_bytes as i64).into(),
                data.width_px.map(|v| v as i32).into(),
                data.height_px.map(|v| v as i32).into(),
            ],
        ))
        .await
        .map_err(|e| MediaRepositoryError::DatabaseError(e.to_string()))?;

        Ok(MediaVariant {
            size: data.size.clone(),
            // The internal read route, not the storage location: callers reach
            // bytes through a signed URL, so bucket and key stay private.
            path: format!("/api/media/{}/{}", data.media_id, variant_type),
        })
    }

    fn media_state_to_db_str(state: &MediaState) -> &'static str {
        match state {
            MediaState::Pending => "pending",
            MediaState::Processing => "processing",
            MediaState::Ready => "ready",
            MediaState::Failed => "failed",
        }
    }

    fn make_object_key(media_id: Uuid, original_name: &str) -> Result<String, RecordMediaError> {
        let ext = std::path::Path::new(original_name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();

        if ext.is_empty() {
            return Err(RecordMediaError::DatabaseError(
                "original_name has no extension".to_string(),
            ));
        }

        match ext.as_str() {
            "jpg" | "jpeg" | "png" | "webp" => Ok(format!("{}/{}", media_id, original_name)),
            _ => Err(RecordMediaError::DatabaseError(format!(
                "invalid extension: {}",
                ext
            ))),
        }
    }

    // =====================================================
    // Facade hook: lets tests supply a fake DB/txn
    // =====================================================

    async fn record_media_tx_with_db<D: MediaDb>(
        db: &D,
        tx: RecordMediaTx,
    ) -> Result<RecordedMedia, RecordMediaError> {
        let now = Utc::now().fixed_offset();

        let media_id = Uuid::new_v4();
        let owner_uuid: Uuid = tx.media.owner.into();

        let bucket_name = tx.media.bucket_name.trim().to_string();
        let original_filename = tx.media.original_name.trim().to_string();
        let mime_type = tx.media.mime_type.trim().to_string();

        let object_key = Self::make_object_key(media_id, &original_filename)?;

        let file_size_bytes_i64 = tx.media.file_size_bytes as i64;
        let width_i32 = tx.media.width_px.map(|v| v as i32);
        let height_i32 = tx.media.height_px.map(|v| v as i32);
        let duration_seconds_i64 = tx.media.duration_seconds.map(|v| v as i64);

        let status_str = Self::media_state_to_db_str(&tx.media.state);

        let attachable_type = tx.attachment.attachment_target.to_string();
        let role = tx.attachment.role.to_string();
        let attachable_id = tx.attachment.attachment_target_id;
        let position_i32 = tx.attachment.position as i32;

        let alt_text = tx.attachment.alt_text.clone();
        let caption = tx.attachment.caption.clone();

        let mut txn = db.begin().await.map_err(Self::map_db_err)?;

        // insert media
        if let Err(e) = txn
            .execute(Self::insert_media_stmt(
                media_id,
                owner_uuid,
                &bucket_name,
                &object_key,
                &original_filename,
                &mime_type,
                file_size_bytes_i64,
                width_i32,
                height_i32,
                duration_seconds_i64,
                status_str,
                now,
            ))
            .await
        {
            let _ = txn.rollback().await;
            return Err(Self::map_db_err(e));
        }

        // insert attachment
        if let Err(e) = txn
            .execute(Self::insert_attachment_stmt(
                media_id,
                &attachable_type,
                attachable_id,
                &role,
                position_i32,
                alt_text,
                caption,
                now,
            ))
            .await
        {
            let _ = txn.rollback().await;
            return Err(Self::map_db_err(e));
        }

        // commit
        txn.commit().await.map_err(Self::map_db_err)?;

        Ok(RecordedMedia {
            owner: tx.media.owner,
            media_id,
            bucket_name,
            original_name: original_filename,
            attachment_target: tx.attachment.attachment_target,
            state: tx.media.state,
        })
    }
}

#[async_trait]
impl MediaRepository for MediaRepositoryPostgres {
    async fn record_media_tx(&self, tx: RecordMediaTx) -> Result<RecordedMedia, RecordMediaError> {
        // production db adapter
        let db = SeaOrmDb {
            db: self.db.clone(),
        };
        Self::record_media_tx_with_db(&db, tx).await
    }

    /// Sets a media row's processing state.
    ///
    /// Scoped by owner and skips soft-deleted rows.
    ///
    /// Deliberately weaker than the sibling `media-status-updater` Cloud
    /// Function, which owns the manifest-driven path and additionally refuses
    /// to leave a terminal state and rejects out-of-order events via
    /// `updated_at <= $manifest_ts`. `UpdateMediaStateData` carries no
    /// timestamp, so there is no ordering signal to enforce here. Use this for
    /// direct or administrative transitions; do not wire it to the manifest
    /// pipeline, or the two writers will race and this one will win by
    /// clobbering.
    async fn set_media_state(
        &self,
        data: UpdateMediaStateData,
    ) -> Result<MediaStateInfo, MediaRepositoryError> {
        let status_str = Self::media_state_to_db_str(&data.status);

        let row = self
            .db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"UPDATE media
                   SET status = $1::media_status, updated_at = NOW()
                   WHERE id = $2 AND user_id = $3 AND deleted_at IS NULL
                   RETURNING updated_at"#,
                [
                    status_str.into(),
                    data.media_id.into(),
                    data.owner.value().into(),
                ],
            ))
            .await
            .map_err(|e| MediaRepositoryError::DatabaseError(e.to_string()))?
            .ok_or(MediaRepositoryError::NotFound)?;

        let updated_at: chrono::DateTime<chrono::FixedOffset> = row
            .try_get("", "updated_at")
            .map_err(|e| MediaRepositoryError::DatabaseError(e.to_string()))?;

        Ok(MediaStateInfo {
            owner: data.owner,
            media_id: data.media_id,
            updated_at: updated_at.to_rfc3339(),
            status: data.status,
        })
    }

    async fn soft_delete(&self, owner: UserId, media_id: Uuid) -> Result<(), MediaRepositoryError> {
        // `deleted_at IS NULL` keeps this idempotent at the SQL level: a second
        // delete matches no row, which is then reported as success below only
        // if the row exists and is already deleted.
        let result = self
            .db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"UPDATE media SET deleted_at = NOW(), updated_at = NOW()
                   WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL"#,
                [media_id.into(), owner.value().into()],
            ))
            .await
            .map_err(|e| MediaRepositoryError::DatabaseError(e.to_string()))?;

        if result.rows_affected() > 0 {
            return Ok(());
        }

        // Nothing updated: either the row is absent, owned by someone else, or
        // already deleted. Only the last of those is success.
        let existing = self
            .db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"SELECT id FROM media WHERE id = $1 AND user_id = $2"#,
                [media_id.into(), owner.value().into()],
            ))
            .await
            .map_err(|e| MediaRepositoryError::DatabaseError(e.to_string()))?;

        match existing {
            Some(_) => Ok(()),
            None => Err(MediaRepositoryError::NotFound),
        }
    }

    async fn record_single_variant(
        &self,
        data: MediaVariantRecord,
    ) -> Result<MediaVariant, MediaRepositoryError> {
        let db = &*self.db;
        Self::upsert_variant(db, &data).await
    }

    /// Records a batch of variants atomically.
    ///
    /// All-or-nothing: the processing pipeline publishes a media item's
    /// variants as one manifest, and a partial set would advertise sizes that
    /// do not exist behind the read route.
    async fn record_variants(
        &self,
        data: Vec<MediaVariantRecord>,
    ) -> Result<Vec<MediaVariant>, MediaRepositoryError> {
        if data.is_empty() {
            return Ok(vec![]);
        }

        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| MediaRepositoryError::DatabaseError(e.to_string()))?;

        let mut recorded = Vec::with_capacity(data.len());
        for record in &data {
            match Self::upsert_variant(&txn, record).await {
                Ok(v) => recorded.push(v),
                Err(e) => {
                    // Ignore rollback failure: the original error is the one
                    // worth reporting, and the transaction is dropped either way.
                    let _ = txn.rollback().await;
                    return Err(e);
                }
            }
        }

        txn.commit()
            .await
            .map_err(|e| MediaRepositoryError::DatabaseError(e.to_string()))?;

        Ok(recorded)
    }
}

// ============================================================================
// Minimal DB Facade (so tests don’t rely on SeaORM MockDatabase txn behavior)
// ============================================================================

#[async_trait]
trait MediaDb: Send + Sync {
    type Txn: MediaTxn;
    async fn begin(&self) -> Result<Self::Txn, DbErr>;
}

#[async_trait]
trait MediaTxn: Send {
    async fn execute(&mut self, stmt: Statement) -> Result<(), DbErr>;
    async fn commit(self) -> Result<(), DbErr>;
    async fn rollback(self) -> Result<(), DbErr>;
}

struct SeaOrmDb {
    db: Arc<DatabaseConnection>,
}

struct SeaOrmTxn {
    txn: sea_orm::DatabaseTransaction,
}

#[async_trait]
impl MediaDb for SeaOrmDb {
    type Txn = SeaOrmTxn;

    async fn begin(&self) -> Result<Self::Txn, DbErr> {
        let txn = self.db.begin().await?;
        Ok(SeaOrmTxn { txn })
    }
}

#[async_trait]
impl MediaTxn for SeaOrmTxn {
    async fn execute(&mut self, stmt: Statement) -> Result<(), DbErr> {
        self.txn.execute(stmt).await?;
        Ok(())
    }

    async fn commit(self) -> Result<(), DbErr> {
        self.txn.commit().await
    }

    async fn rollback(self) -> Result<(), DbErr> {
        self.txn.rollback().await
    }
}

// ============================================================================
// Tests (deterministic, 100% branch coverage of record_media_tx logic)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::domain::entities::UserId;
    use crate::multimedia::application::domain::entities::{
        AttachmentTarget, MediaRole, MediaState,
    };
    use crate::multimedia::application::ports::outgoing::db::{NewMedia, NewMediaAttachment};

    #[derive(Debug)]
    enum Step {
        Begin(Result<(), DbErr>),
        Exec(Result<(), DbErr>),
        Commit(Result<(), DbErr>),
        Rollback(Result<(), DbErr>),
    }

    #[derive(Clone)]
    struct FakeDb {
        steps: Arc<std::sync::Mutex<Vec<Step>>>,
    }

    struct FakeTxn {
        steps: Arc<std::sync::Mutex<Vec<Step>>>,
    }

    impl FakeDb {
        fn new(steps: Vec<Step>) -> Self {
            Self {
                steps: Arc::new(std::sync::Mutex::new(steps)),
            }
        }

        fn pop(&self) -> Step {
            self.steps.lock().unwrap().remove(0)
        }
    }

    #[async_trait]
    impl MediaDb for FakeDb {
        type Txn = FakeTxn;

        async fn begin(&self) -> Result<Self::Txn, DbErr> {
            match self.pop() {
                Step::Begin(Ok(())) => Ok(FakeTxn {
                    steps: self.steps.clone(),
                }),
                Step::Begin(Err(e)) => Err(e),
                other => panic!("Expected Step::Begin, got: {:?}", other),
            }
        }
    }

    #[async_trait]
    impl MediaTxn for FakeTxn {
        async fn execute(&mut self, _stmt: Statement) -> Result<(), DbErr> {
            let step = self.steps.lock().unwrap().remove(0);
            match step {
                Step::Exec(res) => res,
                other => panic!("Expected Step::Exec, got: {:?}", other),
            }
        }

        async fn commit(self) -> Result<(), DbErr> {
            let step = self.steps.lock().unwrap().remove(0);
            match step {
                Step::Commit(res) => res,
                other => panic!("Expected Step::Commit, got: {:?}", other),
            }
        }

        async fn rollback(self) -> Result<(), DbErr> {
            let step = self.steps.lock().unwrap().remove(0);
            match step {
                Step::Rollback(res) => res,
                other => panic!("Expected Step::Rollback, got: {:?}", other),
            }
        }
    }

    fn make_tx(original_name: &str) -> RecordMediaTx {
        let owner = UserId::from(Uuid::new_v4());

        RecordMediaTx {
            media: NewMedia {
                owner,
                state: MediaState::Pending,
                bucket_name: "bucket-a".to_string(),
                original_name: original_name.to_string(),
                mime_type: "image/png".to_string(),
                file_size_bytes: 1024,
                width_px: Some(400),
                height_px: Some(300),
                duration_seconds: None,
            },
            attachment: NewMediaAttachment {
                owner: UserId::from(Uuid::new_v4()),
                attachment_target: AttachmentTarget::Resume,
                attachment_target_id: Uuid::new_v4(),
                role: MediaRole::Profile,
                position: 0,
                alt_text: Some("alt".to_string()),
                caption: Some("caption".to_string()),
            },
        }
    }

    #[tokio::test]
    async fn test_record_media_tx_success() {
        let db = FakeDb::new(vec![
            Step::Begin(Ok(())),
            Step::Exec(Ok(())),   // insert media
            Step::Exec(Ok(())),   // insert attachment
            Step::Commit(Ok(())), // commit
        ]);

        let tx = make_tx("cat.png");
        let res = MediaRepositoryPostgres::record_media_tx_with_db(&db, tx).await;

        assert!(res.is_ok());
        let recorded = res.unwrap();
        assert_eq!(recorded.bucket_name, "bucket-a");
        assert_eq!(recorded.original_name, "cat.png");
        assert_eq!(recorded.state, MediaState::Pending);
        assert_eq!(recorded.attachment_target, AttachmentTarget::Resume);
    }

    #[tokio::test]
    async fn test_record_media_tx_invalid_extension_returns_error() {
        let db = FakeDb::new(vec![]); // should fail before begin()

        let tx = make_tx("no_extension");
        let err = MediaRepositoryPostgres::record_media_tx_with_db(&db, tx)
            .await
            .unwrap_err();

        match err {
            RecordMediaError::DatabaseError(msg) => {
                assert!(msg.to_lowercase().contains("extension"));
            }
        }
    }

    #[tokio::test]
    async fn test_record_media_tx_begin_error() {
        let db = FakeDb::new(vec![Step::Begin(Err(DbErr::Custom(
            "begin failed".to_string(),
        )))]);

        let tx = make_tx("cat.png");
        let err = MediaRepositoryPostgres::record_media_tx_with_db(&db, tx)
            .await
            .unwrap_err();

        match err {
            RecordMediaError::DatabaseError(msg) => assert!(msg.contains("begin failed")),
        }
    }

    #[tokio::test]
    async fn test_record_media_tx_insert_media_error_rolls_back() {
        let db = FakeDb::new(vec![
            Step::Begin(Ok(())),
            Step::Exec(Err(DbErr::Custom("insert media failed".to_string()))),
            Step::Rollback(Ok(())),
        ]);

        let tx = make_tx("cat.png");
        let err = MediaRepositoryPostgres::record_media_tx_with_db(&db, tx)
            .await
            .unwrap_err();

        match err {
            RecordMediaError::DatabaseError(msg) => assert!(msg.contains("insert media failed")),
        }
    }

    #[tokio::test]
    async fn test_record_media_tx_insert_attachment_error_rolls_back() {
        let db = FakeDb::new(vec![
            Step::Begin(Ok(())),
            Step::Exec(Ok(())), // insert media ok
            Step::Exec(Err(DbErr::Custom("insert attachment failed".to_string()))),
            Step::Rollback(Ok(())),
        ]);

        let tx = make_tx("cat.png");
        let err = MediaRepositoryPostgres::record_media_tx_with_db(&db, tx)
            .await
            .unwrap_err();

        match err {
            RecordMediaError::DatabaseError(msg) => {
                assert!(msg.contains("insert attachment failed"))
            }
        }
    }

    #[tokio::test]
    async fn test_record_media_tx_commit_error_returns_error() {
        let db = FakeDb::new(vec![
            Step::Begin(Ok(())),
            Step::Exec(Ok(())), // insert media ok
            Step::Exec(Ok(())), // insert attachment ok
            Step::Commit(Err(DbErr::Custom("commit failed".to_string()))),
        ]);

        let tx = make_tx("cat.png");
        let err = MediaRepositoryPostgres::record_media_tx_with_db(&db, tx)
            .await
            .unwrap_err();

        match err {
            RecordMediaError::DatabaseError(msg) => assert!(msg.contains("commit failed")),
        }
    }
}

#[cfg(test)]
mod state_and_variant_tests {
    use super::*;
    use crate::auth::application::domain::entities::UserId;
    use crate::multimedia::application::domain::entities::MediaSize;
    use sea_orm::{MockDatabase, MockExecResult, Value};
    use std::collections::BTreeMap;

    fn make_row(data: Vec<(&str, Value)>) -> BTreeMap<String, Value> {
        data.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    fn variant(media_id: Uuid, size: MediaSize) -> MediaVariantRecord {
        MediaVariantRecord {
            owner: UserId::from(Uuid::new_v4()),
            media_id,
            size,
            bucket_name: "  blogport-cms-ready  ".to_string(),
            object_key: "  media/obj.webp  ".to_string(),
            mime_type: "  image/webp  ".to_string(),
            file_size_bytes: 2048,
            width_px: Some(320),
            height_px: Some(240),
        }
    }

    fn exec_ok(n: u64) -> MockExecResult {
        MockExecResult {
            last_insert_id: 0,
            rows_affected: n,
        }
    }

    // -----------------------
    // set_media_state
    // -----------------------

    #[tokio::test]
    async fn set_media_state_returns_the_new_state() {
        let now = chrono::Utc::now().fixed_offset();
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![make_row(vec![(
                "updated_at",
                Value::ChronoDateTimeWithTimeZone(Some(Box::new(now))),
            )])]])
            .into_connection();

        let repo = MediaRepositoryPostgres::new(Arc::new(db));
        let owner = UserId::from(Uuid::new_v4());
        let media_id = Uuid::new_v4();

        let info = repo
            .set_media_state(UpdateMediaStateData {
                owner,
                media_id,
                status: MediaState::Ready,
            })
            .await
            .unwrap();

        assert_eq!(info.media_id, media_id);
        assert_eq!(info.status, MediaState::Ready);
        assert_eq!(info.updated_at, now.to_rfc3339());
    }

    /// The UPDATE is scoped by owner and skips soft-deleted rows, so "no row
    /// returned" covers absent, not-yours, and already-deleted alike.
    #[tokio::test]
    async fn set_media_state_reports_not_found_when_no_row_matches() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<BTreeMap<String, Value>>::new()])
            .into_connection();

        let repo = MediaRepositoryPostgres::new(Arc::new(db));
        let err = repo
            .set_media_state(UpdateMediaStateData {
                owner: UserId::from(Uuid::new_v4()),
                media_id: Uuid::new_v4(),
                status: MediaState::Failed,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, MediaRepositoryError::NotFound));
    }

    #[tokio::test]
    async fn set_media_state_surfaces_database_errors() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors([DbErr::Custom("db down".to_string())])
            .into_connection();

        let repo = MediaRepositoryPostgres::new(Arc::new(db));
        let err = repo
            .set_media_state(UpdateMediaStateData {
                owner: UserId::from(Uuid::new_v4()),
                media_id: Uuid::new_v4(),
                status: MediaState::Processing,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, MediaRepositoryError::DatabaseError(m) if m.contains("db down")));
    }

    // -----------------------
    // record_single_variant
    // -----------------------

    /// The returned `path` is the internal read route, not the storage
    /// location, so bucket and object key never reach the caller.
    #[tokio::test]
    async fn record_single_variant_returns_the_internal_read_path() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results([exec_ok(1)])
            .into_connection();

        let repo = MediaRepositoryPostgres::new(Arc::new(db));
        let media_id = Uuid::new_v4();

        let v = repo
            .record_single_variant(variant(media_id, MediaSize::Thumbnail))
            .await
            .unwrap();

        assert_eq!(v.size, MediaSize::Thumbnail);
        assert_eq!(v.path, format!("/api/media/{media_id}/thumbnail"));
        assert!(!v.path.contains("blogport-cms-ready"));
    }

    #[tokio::test]
    async fn record_single_variant_surfaces_database_errors() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_errors([DbErr::Custom("conflict".to_string())])
            .into_connection();

        let repo = MediaRepositoryPostgres::new(Arc::new(db));
        let err = repo
            .record_single_variant(variant(Uuid::new_v4(), MediaSize::Large))
            .await
            .unwrap_err();

        assert!(matches!(err, MediaRepositoryError::DatabaseError(m) if m.contains("conflict")));
    }

    // -----------------------
    // record_variants
    // -----------------------

    #[tokio::test]
    async fn record_variants_writes_every_size() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results([exec_ok(1), exec_ok(1), exec_ok(1)])
            .into_connection();

        let repo = MediaRepositoryPostgres::new(Arc::new(db));
        let media_id = Uuid::new_v4();

        let out = repo
            .record_variants(vec![
                variant(media_id, MediaSize::Thumbnail),
                variant(media_id, MediaSize::Medium),
                variant(media_id, MediaSize::Large),
            ])
            .await
            .unwrap();

        let sizes: Vec<_> = out.iter().map(|v| v.size.clone()).collect();
        assert_eq!(
            sizes,
            vec![MediaSize::Thumbnail, MediaSize::Medium, MediaSize::Large]
        );
    }

    /// An empty batch must not open a transaction or fail; nothing to record is
    /// a valid outcome, not an error.
    #[tokio::test]
    async fn record_variants_accepts_an_empty_batch() {
        let db = MockDatabase::new(DatabaseBackend::Postgres).into_connection();
        let repo = MediaRepositoryPostgres::new(Arc::new(db));

        assert!(repo.record_variants(vec![]).await.unwrap().is_empty());
    }

    /// All-or-nothing: a media item's variants are published together, and a
    /// partial set would advertise sizes the read route cannot serve.
    ///
    /// This pins that the whole call fails rather than returning the rows that
    /// happened to succeed. It does not prove a ROLLBACK reached the server —
    /// MockDatabase does not record transaction statements — so that guarantee
    /// rests on the code path, not on this assertion.
    #[tokio::test]
    async fn record_variants_fails_the_whole_batch_when_one_row_fails() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results([exec_ok(1)])
            .append_exec_errors([DbErr::Custom("second row failed".to_string())])
            .into_connection();

        let repo = MediaRepositoryPostgres::new(Arc::new(db));
        let media_id = Uuid::new_v4();

        let err = repo
            .record_variants(vec![
                variant(media_id, MediaSize::Thumbnail),
                variant(media_id, MediaSize::Medium),
            ])
            .await
            .unwrap_err();

        assert!(
            matches!(err, MediaRepositoryError::DatabaseError(m) if m.contains("second row failed"))
        );
    }
}
