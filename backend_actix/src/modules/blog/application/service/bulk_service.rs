//! Applies one lifecycle or topic operation across many posts.
//!
//! Composes the single-item use cases rather than reaching for the repository,
//! for two reasons. Their rules — owner scoping above all — apply unchanged, so
//! there is no second implementation to keep in step. And a bulk call becomes
//! exactly N of the calls the console already makes, which is what makes the
//! per-item outcomes meaningful.

use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::blog::application::ports::incoming::use_cases::{
    ArchiveBlogPostError, ArchiveBlogPostUseCase, AttachBlogPostTopicUseCase, BlogBulkOp,
    BlogPostTopicError, BulkBlogPostsUseCase, DetachBlogPostTopicUseCase,
    HardDeleteBlogPostUseCase, RestoreBlogPostUseCase,
};
use crate::shared::api::{prepare_ids, BulkOutcome, BulkRequestError, ErrorCode};

/// Implements the corresponding use-case contract.
pub struct BulkBlogPostsService {
    archive: Arc<dyn ArchiveBlogPostUseCase + Send + Sync>,
    restore: Arc<dyn RestoreBlogPostUseCase + Send + Sync>,
    hard_delete: Arc<dyn HardDeleteBlogPostUseCase + Send + Sync>,
    attach_topic: Arc<dyn AttachBlogPostTopicUseCase + Send + Sync>,
    detach_topic: Arc<dyn DetachBlogPostTopicUseCase + Send + Sync>,
}

impl BulkBlogPostsService {
    /// Builds it from the single-item use cases it fans out to.
    pub fn new(
        archive: Arc<dyn ArchiveBlogPostUseCase + Send + Sync>,
        restore: Arc<dyn RestoreBlogPostUseCase + Send + Sync>,
        hard_delete: Arc<dyn HardDeleteBlogPostUseCase + Send + Sync>,
        attach_topic: Arc<dyn AttachBlogPostTopicUseCase + Send + Sync>,
        detach_topic: Arc<dyn DetachBlogPostTopicUseCase + Send + Sync>,
    ) -> Self {
        Self {
            archive,
            restore,
            hard_delete,
            attach_topic,
            detach_topic,
        }
    }
}

fn lifecycle_failure(e: ArchiveBlogPostError) -> (ErrorCode, String) {
    match e {
        ArchiveBlogPostError::NotFound => (ErrorCode::PostNotFound, e.to_string()),
        ArchiveBlogPostError::RepositoryError(_) => (ErrorCode::InternalError, e.to_string()),
    }
}

fn topic_failure(e: BlogPostTopicError) -> (ErrorCode, String) {
    match e {
        BlogPostTopicError::PostNotFound => (ErrorCode::PostNotFound, e.to_string()),
        BlogPostTopicError::TopicNotFound => (ErrorCode::TopicNotFound, e.to_string()),
        BlogPostTopicError::RepositoryError(_) => (ErrorCode::InternalError, e.to_string()),
    }
}

#[async_trait]
impl BulkBlogPostsUseCase for BulkBlogPostsService {
    async fn execute(
        &self,
        owner: UserId,
        op: BlogBulkOp,
        ids: Vec<Uuid>,
    ) -> Result<BulkOutcome, BulkRequestError> {
        let ids = prepare_ids(ids)?;
        let mut outcome = BulkOutcome::default();

        // Sequential on purpose. Each item is a database write, and running a
        // hundred concurrently would exhaust the pool for every other request
        // in flight. It also keeps `succeeded` in request order, which the
        // console needs to report progress against its own selection.
        for id in ids {
            let result = match &op {
                BlogBulkOp::Archive => self
                    .archive
                    .execute(owner, id)
                    .await
                    .map_err(lifecycle_failure),
                BlogBulkOp::Restore => self
                    .restore
                    .execute(owner, id)
                    .await
                    .map_err(lifecycle_failure),
                BlogBulkOp::HardDelete => self
                    .hard_delete
                    .execute(owner, id)
                    .await
                    .map_err(lifecycle_failure),
                BlogBulkOp::AttachTopic { topic_id } => self
                    .attach_topic
                    .execute(owner, id, *topic_id)
                    .await
                    .map_err(topic_failure),
                BlogBulkOp::DetachTopic { topic_id } => self
                    .detach_topic
                    .execute(owner, id, *topic_id)
                    .await
                    .map_err(topic_failure),
            };

            match result {
                Ok(()) => outcome.succeed(id),
                Err((code, message)) => outcome.fail(id, code, message),
            }
        }

        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Records what it was asked to do, and fails the ids it was told to.
    #[derive(Default)]
    struct SpyOp {
        seen: Mutex<Vec<(Uuid, Option<Uuid>)>>,
        missing: Vec<Uuid>,
        broken: Vec<Uuid>,
    }

    impl SpyOp {
        fn lifecycle(&self, id: Uuid) -> Result<(), ArchiveBlogPostError> {
            self.seen.lock().unwrap().push((id, None));
            if self.missing.contains(&id) {
                return Err(ArchiveBlogPostError::NotFound);
            }
            if self.broken.contains(&id) {
                return Err(ArchiveBlogPostError::RepositoryError("pool gone".into()));
            }
            Ok(())
        }

        fn topic(&self, id: Uuid, topic_id: Uuid) -> Result<(), BlogPostTopicError> {
            self.seen.lock().unwrap().push((id, Some(topic_id)));
            if self.missing.contains(&id) {
                return Err(BlogPostTopicError::PostNotFound);
            }
            Ok(())
        }
    }

    #[async_trait]
    impl ArchiveBlogPostUseCase for SpyOp {
        async fn execute(&self, _o: UserId, id: Uuid) -> Result<(), ArchiveBlogPostError> {
            self.lifecycle(id)
        }
    }
    #[async_trait]
    impl RestoreBlogPostUseCase for SpyOp {
        async fn execute(&self, _o: UserId, id: Uuid) -> Result<(), ArchiveBlogPostError> {
            self.lifecycle(id)
        }
    }
    #[async_trait]
    impl HardDeleteBlogPostUseCase for SpyOp {
        async fn execute(&self, _o: UserId, id: Uuid) -> Result<(), ArchiveBlogPostError> {
            self.lifecycle(id)
        }
    }
    #[async_trait]
    impl AttachBlogPostTopicUseCase for SpyOp {
        async fn execute(
            &self,
            _o: UserId,
            id: Uuid,
            topic_id: Uuid,
        ) -> Result<(), BlogPostTopicError> {
            self.topic(id, topic_id)
        }
    }
    #[async_trait]
    impl DetachBlogPostTopicUseCase for SpyOp {
        async fn execute(
            &self,
            _o: UserId,
            id: Uuid,
            topic_id: Uuid,
        ) -> Result<(), BlogPostTopicError> {
            self.topic(id, topic_id)
        }
    }

    fn service(spy: Arc<SpyOp>) -> BulkBlogPostsService {
        BulkBlogPostsService::new(
            Arc::clone(&spy) as Arc<dyn ArchiveBlogPostUseCase + Send + Sync>,
            Arc::clone(&spy) as Arc<dyn RestoreBlogPostUseCase + Send + Sync>,
            Arc::clone(&spy) as Arc<dyn HardDeleteBlogPostUseCase + Send + Sync>,
            Arc::clone(&spy) as Arc<dyn AttachBlogPostTopicUseCase + Send + Sync>,
            spy as Arc<dyn DetachBlogPostTopicUseCase + Send + Sync>,
        )
    }

    fn owner() -> UserId {
        UserId::from(Uuid::new_v4())
    }

    #[tokio::test]
    async fn every_id_is_attempted_and_reported() {
        let ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
        let spy = Arc::new(SpyOp::default());

        let outcome = service(Arc::clone(&spy))
            .execute(owner(), BlogBulkOp::Archive, ids.clone())
            .await
            .unwrap();

        assert_eq!(outcome.succeeded, ids);
        assert!(outcome.is_complete());
        assert_eq!(spy.seen.lock().unwrap().len(), 3);
    }

    /// The point of a bulk endpoint: one bad id must not sink the batch, and
    /// the caller must be able to tell which one it was.
    #[tokio::test]
    async fn one_failure_does_not_stop_the_rest() {
        let good_a = Uuid::new_v4();
        let missing = Uuid::new_v4();
        let good_b = Uuid::new_v4();

        let spy = Arc::new(SpyOp {
            missing: vec![missing],
            ..Default::default()
        });

        let outcome = service(spy)
            .execute(
                owner(),
                BlogBulkOp::HardDelete,
                vec![good_a, missing, good_b],
            )
            .await
            .unwrap();

        assert_eq!(outcome.succeeded, vec![good_a, good_b]);
        assert_eq!(outcome.failed.len(), 1);
        assert_eq!(outcome.failed[0].id, missing);
        assert_eq!(outcome.failed[0].code, ErrorCode::PostNotFound);
    }

    /// A post owned by someone else comes back as NotFound from the composed
    /// use case, and must surface as an ordinary per-item failure — never as a
    /// different code that would confirm the post exists.
    #[tokio::test]
    async fn another_authors_post_is_reported_as_not_found() {
        let theirs = Uuid::new_v4();
        let spy = Arc::new(SpyOp {
            missing: vec![theirs],
            ..Default::default()
        });

        let outcome = service(spy)
            .execute(owner(), BlogBulkOp::Archive, vec![theirs])
            .await
            .unwrap();

        assert!(outcome.succeeded.is_empty());
        assert_eq!(outcome.failed[0].code, ErrorCode::PostNotFound);
    }

    /// A store failure is not a missing post. Collapsing the two would tell a
    /// console to drop rows from its list during an outage.
    #[tokio::test]
    async fn a_store_failure_is_distinguishable_from_a_missing_post() {
        let broken = Uuid::new_v4();
        let spy = Arc::new(SpyOp {
            broken: vec![broken],
            ..Default::default()
        });

        let outcome = service(spy)
            .execute(owner(), BlogBulkOp::Archive, vec![broken])
            .await
            .unwrap();

        assert_eq!(outcome.failed[0].code, ErrorCode::InternalError);
    }

    #[tokio::test]
    async fn a_topic_operation_passes_the_topic_through() {
        let post = Uuid::new_v4();
        let topic = Uuid::new_v4();
        let spy = Arc::new(SpyOp::default());

        service(Arc::clone(&spy))
            .execute(
                owner(),
                BlogBulkOp::AttachTopic { topic_id: topic },
                vec![post],
            )
            .await
            .unwrap();

        assert_eq!(spy.seen.lock().unwrap()[0], (post, Some(topic)));
    }

    /// The cap is enforced before any item is touched, so an oversized batch
    /// cannot half-apply.
    #[tokio::test]
    async fn an_oversized_batch_touches_nothing() {
        let ids: Vec<Uuid> = (0..crate::shared::api::MAX_BULK_IDS + 1)
            .map(|_| Uuid::new_v4())
            .collect();
        let spy = Arc::new(SpyOp::default());

        let err = service(Arc::clone(&spy))
            .execute(owner(), BlogBulkOp::HardDelete, ids)
            .await
            .unwrap_err();

        assert!(matches!(err, BulkRequestError::TooLarge(_)));
        assert!(
            spy.seen.lock().unwrap().is_empty(),
            "no post may be touched when the batch is rejected"
        );
    }

    #[tokio::test]
    async fn a_repeated_id_is_applied_once() {
        let id = Uuid::new_v4();
        let spy = Arc::new(SpyOp::default());

        let outcome = service(Arc::clone(&spy))
            .execute(owner(), BlogBulkOp::HardDelete, vec![id, id, id])
            .await
            .unwrap();

        assert_eq!(outcome.succeeded, vec![id]);
        assert_eq!(spy.seen.lock().unwrap().len(), 1);
    }
}
