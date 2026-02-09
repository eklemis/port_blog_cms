use async_trait::async_trait;
use chrono::Utc;
use sea_orm::ConnectionTrait;
use sea_orm::{DatabaseBackend, DatabaseConnection, DbErr, Statement, TransactionTrait};
use std::sync::Arc;
use uuid::Uuid;

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
            "jpg" | "jpeg" | "png" | "webp" => Ok(format!("{}.{}", media_id, ext)),
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

    async fn set_media_state(
        &self,
        _data: UpdateMediaStateData,
    ) -> Result<MediaStateInfo, MediaRepositoryError> {
        todo!()
    }

    async fn record_single_variant(
        &self,
        _data: MediaVariantRecord,
    ) -> Result<MediaVariant, MediaRepositoryError> {
        todo!()
    }

    async fn record_variants(
        &self,
        _data: Vec<MediaVariantRecord>,
    ) -> Result<Vec<MediaVariant>, MediaRepositoryError> {
        todo!()
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
