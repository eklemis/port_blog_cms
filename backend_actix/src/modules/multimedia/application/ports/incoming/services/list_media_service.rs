use async_trait::async_trait;

use crate::multimedia::application::ports::{
    incoming::use_cases::{ListMediaCommand, ListMediaError, ListMediaUseCase, MediaItem},
    outgoing::db::MediaQuery,
};

pub struct ListMediaService<Q>
where
    Q: MediaQuery,
{
    query: Q,
}

impl<Q> ListMediaService<Q>
where
    Q: MediaQuery,
{
    pub fn new(query: Q) -> Self {
        Self { query }
    }
}

#[async_trait]
impl<Q> ListMediaUseCase for ListMediaService<Q>
where
    Q: MediaQuery,
{
    async fn execute(&self, command: ListMediaCommand) -> Result<Vec<MediaItem>, ListMediaError> {
        let attachments = self
            .query
            .list_by_target(command.owner, command.attachment_target)
            .await?;
        let items = attachments
            .into_iter()
            .map(MediaItem::from_media_attachment)
            .collect();
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    use crate::auth::application::domain::entities::UserId;
    use crate::multimedia::application::domain::entities::{
        AttachmentTarget, MediaRole, MediaState, MediaStateInfo,
    };
    use crate::multimedia::application::ports::incoming::use_cases::ListMediaCommand;
    use crate::multimedia::application::ports::outgoing::db::{
        MediaAttachment, MediaQueryError, StoredVariant,
    };

    /// Simple mock query that:
    /// - returns a pre-configured result for list_by_target
    /// - records calls (owner + target) so tests can assert the correct args were used
    #[derive(Clone)]
    struct MockMediaQuery {
        list_result: Arc<Mutex<Result<Vec<MediaAttachment>, MediaQueryError>>>,
        calls: Arc<Mutex<Vec<(UserId, AttachmentTarget)>>>,
    }

    impl MockMediaQuery {
        fn success(attachments: Vec<MediaAttachment>) -> Self {
            Self {
                list_result: Arc::new(Mutex::new(Ok(attachments))),
                calls: Arc::new(Mutex::new(vec![])),
            }
        }

        fn failure(err: MediaQueryError) -> Self {
            Self {
                list_result: Arc::new(Mutex::new(Err(err))),
                calls: Arc::new(Mutex::new(vec![])),
            }
        }

        fn take_calls(&self) -> Vec<(UserId, AttachmentTarget)> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl MediaQuery for MockMediaQuery {
        async fn get_state(&self, _media_id: Uuid) -> Result<MediaStateInfo, MediaQueryError> {
            unimplemented!("not needed for these tests")
        }

        async fn list_by_target(
            &self,
            owner: UserId,
            target: AttachmentTarget,
        ) -> Result<Vec<MediaAttachment>, MediaQueryError> {
            self.calls.lock().unwrap().push((owner, target));
            // Return the stored result (clone it so repeated calls are safe if you add more tests later).
            self.list_result.lock().unwrap().clone()
        }

        async fn get_attachment_info(
            &self,
            _media_id: Uuid,
        ) -> Result<MediaAttachment, MediaQueryError> {
            unimplemented!("not needed for these tests")
        }
    }

    fn sample_attachment(owner: UserId, target: AttachmentTarget) -> MediaAttachment {
        // Build a MediaAttachment that exercises every mapped field in MediaItem::from_media_attachment.
        // Adjust field names/types if your MediaAttachment differs slightly.
        MediaAttachment {
            owner,
            media_id: Uuid::new_v4(),
            original_filename: "photo.png".to_string(),
            status: MediaState::Ready, // <-- adjust if needed
            attachment_target: target,
            attachment_target_id: owner.into(), // <-- adjust if attachment_target_id type differs
            role: MediaRole::Avatar,
            position: 1,
            alt_text: "alt".to_string(),
            caption: "caption".to_string(),
            variants: Vec::<StoredVariant>::new(),
        }
    }

    // Pick ONE test attribute that matches your project.
    // If you already use tokio in tests, keep #[tokio::test].
    // If tokio macros aren't enabled, switch to #[actix_web::test] (actix is already in your deps).
    #[tokio::test]
    async fn execute_success_maps_attachments_to_items_and_calls_query_with_expected_args() {
        let owner = UserId::from(Uuid::new_v4());
        let target = AttachmentTarget::Project; // <-- adjust to a real variant in your enum

        let a1 = sample_attachment(owner, target.clone());
        let a2 = sample_attachment(owner, target.clone());

        let query = MockMediaQuery::success(vec![a1.clone(), a2.clone()]);
        let service = ListMediaService::new(query.clone());

        let command = ListMediaCommand {
            owner,
            attachment_target: target.clone(),
        };

        let items = service.execute(command).await.expect("expected Ok");

        // 1) Query called with expected args
        let calls = query.take_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, owner);
        assert_eq!(calls[0].1, target);

        // 2) Mapping happened (compare field-by-field via MediaItem::from_media_attachment)
        assert_eq!(items.len(), 2);

        let expected_1 = MediaItem::from_media_attachment(a1);
        let expected_2 = MediaItem::from_media_attachment(a2);

        assert_eq!(items[0].media_id, expected_1.media_id);
        assert_eq!(items[0].original_filename, expected_1.original_filename);
        assert_eq!(items[0].status, expected_1.status);
        assert_eq!(items[0].attachment_target, expected_1.attachment_target);
        assert_eq!(
            items[0].attachment_target_id,
            expected_1.attachment_target_id
        );
        assert_eq!(items[0].role, expected_1.role);
        assert_eq!(items[0].position, expected_1.position);
        assert_eq!(items[0].alt_text, expected_1.alt_text);
        assert_eq!(items[0].caption, expected_1.caption);

        assert_eq!(items[1].media_id, expected_2.media_id);
        assert_eq!(items[1].original_filename, expected_2.original_filename);
        assert_eq!(items[1].status, expected_2.status);
        assert_eq!(items[1].attachment_target, expected_2.attachment_target);
        assert_eq!(
            items[1].attachment_target_id,
            expected_2.attachment_target_id
        );
        assert_eq!(items[1].role, expected_2.role);
        assert_eq!(items[1].position, expected_2.position);
        assert_eq!(items[1].alt_text, expected_2.alt_text);
        assert_eq!(items[1].caption, expected_2.caption);
    }

    #[tokio::test]
    async fn execute_error_propagates_and_converts_query_error_into_list_media_error() {
        let owner = UserId::from(Uuid::new_v4());
        let target = AttachmentTarget::Project; // <-- adjust

        // Pick/construct a real MediaQueryError variant from your codebase:
        let query_err = MediaQueryError::MediaNotFound;
        let query = MockMediaQuery::failure(query_err);

        let service = ListMediaService::new(query.clone());

        let command = ListMediaCommand {
            owner,
            attachment_target: target.clone(),
        };

        let err = service.execute(command).await.expect_err("expected Err");

        // This asserts the error path is taken and conversion exists.
        // If ListMediaError has a specific variant for query errors, match it here.
        // Example:
        // match err {
        //     ListMediaError::QueryError(_) => {}
        //     other => panic!("unexpected error: {:?}", other),
        // }

        // Minimum assertion (still ensures path executed):
        let _ = err;

        // Also ensure query was called once with expected args
        let calls = query.take_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, owner);
        assert_eq!(calls[0].1, target);
    }
}
