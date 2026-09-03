//! Minting, reading, revoking and resolving draft preview links.

use async_trait::async_trait;
use chrono::{Duration, Utc};
use rand::RngCore;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::auth::application::ports::outgoing::UserQuery;
use crate::blog::application::ports::incoming::use_cases::{
    DraftPreviewError, DraftPreviewState, GetDraftPreviewUseCase, PreviewResolution,
    ReadDraftPreviewUseCase, ReadPreviewMediaUseCase, RevokeDraftPreviewUseCase, ShareDraftUseCase,
    DRAFT_PREVIEW_TTL_DAYS,
};
use crate::blog::application::ports::outgoing::{
    BlogPostQuery, DraftPreview, DraftPreviewStore, PreviewMediaResolver,
};

/// Length of the shareable secret, in bytes before encoding.
///
/// 32 bytes of CSPRNG output. The link is the only thing standing between an
/// unpublished draft and the internet, so it must not be guessable and must not
/// be derived from the post id — which appears in console URLs and would make
/// every draft's address recoverable by anyone who has seen the author's
/// screen.
const TOKEN_BYTES: usize = 32;

fn mint_token() -> String {
    let mut bytes = [0u8; TOKEN_BYTES];
    rand::thread_rng().fill_bytes(&mut bytes);

    // Hex rather than base64: URL-safe with no encoding variant to get wrong,
    // and it saves taking a runtime dependency for one call. The link is a
    // little longer, which costs nothing — nobody types it.
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn state_of(preview: DraftPreview) -> DraftPreviewState {
    DraftPreviewState {
        expired: preview.expires_at <= Utc::now(),
        token: preview.token,
        expires_at: preview.expires_at,
        created_at: preview.created_at,
    }
}

/// Implements the corresponding use-case contract.
pub struct ShareDraftService<S> {
    previews: S,
}

impl<S> ShareDraftService<S> {
    /// Builds it from the ports it depends on.
    pub fn new(previews: S) -> Self {
        Self { previews }
    }
}

#[async_trait]
impl<S> ShareDraftUseCase for ShareDraftService<S>
where
    S: DraftPreviewStore + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        post_id: Uuid,
    ) -> Result<DraftPreviewState, DraftPreviewError> {
        let expires_at = Utc::now() + Duration::days(DRAFT_PREVIEW_TTL_DAYS);

        // The token is only used when there is no row yet; the store keeps the
        // existing one on a renew, so a bookmark survives.
        let preview = self
            .previews
            .upsert(owner.value(), post_id, expires_at, &mint_token())
            .await?;

        Ok(state_of(preview))
    }
}

/// Implements the corresponding use-case contract.
pub struct GetDraftPreviewService<S> {
    previews: S,
}

impl<S> GetDraftPreviewService<S> {
    /// Builds it from the ports it depends on.
    pub fn new(previews: S) -> Self {
        Self { previews }
    }
}

#[async_trait]
impl<S> GetDraftPreviewUseCase for GetDraftPreviewService<S>
where
    S: DraftPreviewStore + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        post_id: Uuid,
    ) -> Result<DraftPreviewState, DraftPreviewError> {
        self.previews
            .find_for_post(owner.value(), post_id)
            .await?
            .map(state_of)
            .ok_or(DraftPreviewError::NotShared)
    }
}

/// Implements the corresponding use-case contract.
pub struct RevokeDraftPreviewService<S> {
    previews: S,
}

impl<S> RevokeDraftPreviewService<S> {
    /// Builds it from the ports it depends on.
    pub fn new(previews: S) -> Self {
        Self { previews }
    }
}

#[async_trait]
impl<S> RevokeDraftPreviewUseCase for RevokeDraftPreviewService<S>
where
    S: DraftPreviewStore + Send + Sync,
{
    async fn execute(&self, owner: UserId, post_id: Uuid) -> Result<(), DraftPreviewError> {
        self.previews.revoke(owner.value(), post_id).await?;
        Ok(())
    }
}

/// Resolves a preview token for a reader with no account.
pub struct ReadDraftPreviewService<S, Q, U> {
    previews: S,
    posts: Q,
    users: U,
}

impl<S, Q, U> ReadDraftPreviewService<S, Q, U> {
    /// Builds it from the ports it depends on.
    pub fn new(previews: S, posts: Q, users: U) -> Self {
        Self {
            previews,
            posts,
            users,
        }
    }
}

#[async_trait]
impl<S, Q, U> ReadDraftPreviewUseCase for ReadDraftPreviewService<S, Q, U>
where
    S: DraftPreviewStore + Send + Sync,
    Q: BlogPostQuery + Send + Sync,
    U: UserQuery + Send + Sync,
{
    async fn execute(&self, token: &str) -> Result<PreviewResolution, DraftPreviewError> {
        let now = Utc::now();

        // Unknown, revoked and expired all land here as `None`, and all become
        // the same error. Distinguishing them would let a holder of a dead link
        // learn whether the draft still exists.
        let live = self
            .previews
            .find_live_by_token(token, now)
            .await?
            .ok_or(DraftPreviewError::PostNotFound)?;

        // Read as the post's owner. The token is the authorisation; the reader
        // has no account to scope by.
        let owner_id = live.owner_id;

        let view = self
            .posts
            .get_by_id(UserId::from(owner_id), live.preview.post_id)
            .await
            .map_err(|e| DraftPreviewError::RepositoryError(e.to_string()))?;

        // Read live rather than snapshotted at mint time: the author keeps
        // editing after sharing, and a reviewer refreshing should see the work
        // as it now stands.
        //
        // A post published since the link was shared redirects to its public
        // page. Reporting "expired" for something anyone can now read is the
        // worst moment this feature could produce.
        match view.post.published_at {
            Some(published_at) if published_at <= now => {
                let username = self.username_of(owner_id).await?;
                Ok(PreviewResolution::Published {
                    username,
                    slug: view.post.slug,
                })
            }
            // `None` is a draft, and a future timestamp is scheduled. Both are
            // still unpublished, so both get the preview.
            _ => Ok(PreviewResolution::Draft(Box::new(view))),
        }
    }
}

impl<S, Q, U> ReadDraftPreviewService<S, Q, U>
where
    S: DraftPreviewStore + Send + Sync,
    Q: BlogPostQuery + Send + Sync,
    U: UserQuery + Send + Sync,
{
    async fn username_of(&self, user_id: Uuid) -> Result<String, DraftPreviewError> {
        self.users
            .find_by_id(user_id)
            .await
            .map_err(|e| DraftPreviewError::RepositoryError(e.to_string()))?
            .map(|u| u.username)
            .ok_or(DraftPreviewError::PostNotFound)
    }
}

/// Resolves one image on a previewed draft.
///
/// The token is checked on every image, not once per page. A preview page
/// fetches its images as separate requests, so each one has to carry the
/// capability itself — and revoking the link has to stop the images too, not
/// just the text.
pub struct ReadPreviewMediaService<S> {
    previews: S,
    media: Arc<dyn PreviewMediaResolver>,
}

impl<S> ReadPreviewMediaService<S> {
    /// Builds it from the ports it depends on.
    pub fn new(previews: S, media: Arc<dyn PreviewMediaResolver>) -> Self {
        Self { previews, media }
    }
}

#[async_trait]
impl<S> ReadPreviewMediaUseCase for ReadPreviewMediaService<S>
where
    S: DraftPreviewStore + Send + Sync,
{
    async fn execute(
        &self,
        token: &str,
        media_id: Uuid,
        size: &str,
    ) -> Result<String, DraftPreviewError> {
        let live = self
            .previews
            .find_live_by_token(token, Utc::now())
            .await?
            .ok_or(DraftPreviewError::PostNotFound)?;

        // Scoped to the post the token opens. Without this the token would be
        // a key to every image in the system rather than to one draft's.
        self.media
            .resolve(live.preview.post_id, media_id, size)
            .await
            .map_err(DraftPreviewError::RepositoryError)?
            .ok_or(DraftPreviewError::PostNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::ports::outgoing::user_query::{UserQueryError, UserQueryResult};
    use crate::blog::application::ports::outgoing::{
        BlogPageRequest, BlogPageResult, BlogPostCard, BlogPostListFilter, BlogPostQueryError,
        BlogPostSort, BlogPostView, DraftPreviewStoreError, LivePreview,
    };
    use crate::blog::domain::entities::{BlogPost, BlogPostTopic};
    use chrono::DateTime;
    use std::sync::Arc;
    use std::sync::Mutex;

    fn a_preview(post_id: Uuid, expires_at: DateTime<Utc>) -> DraftPreview {
        DraftPreview {
            post_id,
            token: "tok".into(),
            expires_at,
            created_at: Utc::now() - Duration::days(1),
        }
    }

    #[derive(Default)]
    struct FakeStore {
        row: Mutex<Option<DraftPreview>>,
        owner: Option<Uuid>,
        missing_post: bool,
    }

    #[async_trait]
    impl DraftPreviewStore for FakeStore {
        async fn upsert(
            &self,
            _owner: Uuid,
            post_id: Uuid,
            expires_at: DateTime<Utc>,
            new_token: &str,
        ) -> Result<DraftPreview, DraftPreviewStoreError> {
            if self.missing_post {
                return Err(DraftPreviewStoreError::PostNotFound);
            }
            let mut row = self.row.lock().unwrap();
            let next = match row.clone() {
                // Mirrors ON CONFLICT DO UPDATE SET expires_at: the token and
                // created_at survive a renewal.
                Some(existing) => DraftPreview {
                    expires_at,
                    ..existing
                },
                None => DraftPreview {
                    post_id,
                    token: new_token.to_string(),
                    expires_at,
                    created_at: Utc::now(),
                },
            };
            *row = Some(next.clone());
            Ok(next)
        }

        async fn find_for_post(
            &self,
            _owner: Uuid,
            _post_id: Uuid,
        ) -> Result<Option<DraftPreview>, DraftPreviewStoreError> {
            Ok(self.row.lock().unwrap().clone())
        }

        async fn revoke(&self, _owner: Uuid, _post_id: Uuid) -> Result<(), DraftPreviewStoreError> {
            *self.row.lock().unwrap() = None;
            Ok(())
        }

        async fn find_live_by_token(
            &self,
            _token: &str,
            now: DateTime<Utc>,
        ) -> Result<Option<LivePreview>, DraftPreviewStoreError> {
            let row = self.row.lock().unwrap().clone();
            Ok(row
                .filter(|p| p.expires_at > now)
                .map(|preview| LivePreview {
                    owner_id: self.owner.unwrap_or_else(Uuid::new_v4),
                    preview,
                }))
        }
    }

    struct FakePosts {
        published_at: Option<DateTime<Utc>>,
    }

    #[async_trait]
    impl BlogPostQuery for FakePosts {
        async fn get_by_id(
            &self,
            owner: UserId,
            post_id: Uuid,
        ) -> Result<BlogPostView, BlogPostQueryError> {
            Ok(BlogPostView {
                post: BlogPost {
                    id: post_id,
                    user_id: owner.value(),
                    title: "Draft".into(),
                    slug: "draft-slug".into(),
                    excerpt: None,
                    content: "body".into(),
                    published_at: self.published_at,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
                topics: Vec::<BlogPostTopic>::new(),
                media: vec![],
            })
        }
        async fn list_by_owner(
            &self,
            _o: UserId,
            _f: BlogPostListFilter,
            _s: BlogPostSort,
            _p: BlogPageRequest,
        ) -> Result<BlogPageResult<BlogPostCard>, BlogPostQueryError> {
            unimplemented!()
        }
        async fn get_published_by_slug(
            &self,
            _o: UserId,
            _s: &str,
        ) -> Result<BlogPostView, BlogPostQueryError> {
            unimplemented!()
        }
        async fn list_published(
            &self,
            _o: UserId,
            _f: BlogPostListFilter,
            _s: BlogPostSort,
            _p: BlogPageRequest,
        ) -> Result<BlogPageResult<BlogPostCard>, BlogPostQueryError> {
            unimplemented!()
        }
        async fn slug_exists(&self, _o: UserId, _s: &str) -> Result<bool, BlogPostQueryError> {
            unimplemented!()
        }
        async fn get_topics(&self, _p: Uuid) -> Result<Vec<BlogPostTopic>, BlogPostQueryError> {
            unimplemented!()
        }
    }

    struct FakeUsers;

    #[async_trait]
    impl UserQuery for FakeUsers {
        async fn find_by_id(&self, id: Uuid) -> Result<Option<UserQueryResult>, UserQueryError> {
            Ok(Some(UserQueryResult {
                id,
                email: "jane@example.com".into(),
                username: "janedoe".into(),
                password_hash: "h".into(),
                full_name: "Jane".into(),
                bio: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                is_verified: true,
                is_deleted: false,
            }))
        }
        async fn find_by_email(&self, _e: &str) -> Result<Option<UserQueryResult>, UserQueryError> {
            unimplemented!()
        }
        async fn find_by_username(
            &self,
            _u: &str,
        ) -> Result<Option<UserQueryResult>, UserQueryError> {
            unimplemented!()
        }
    }

    fn reader(
        store: Arc<FakeStore>,
        published_at: Option<DateTime<Utc>>,
    ) -> ReadDraftPreviewService<Arc<FakeStore>, FakePosts, FakeUsers> {
        ReadDraftPreviewService::new(store, FakePosts { published_at }, FakeUsers)
    }

    #[async_trait]
    impl DraftPreviewStore for Arc<FakeStore> {
        async fn upsert(
            &self,
            o: Uuid,
            p: Uuid,
            e: DateTime<Utc>,
            t: &str,
        ) -> Result<DraftPreview, DraftPreviewStoreError> {
            (**self).upsert(o, p, e, t).await
        }
        async fn find_for_post(
            &self,
            o: Uuid,
            p: Uuid,
        ) -> Result<Option<DraftPreview>, DraftPreviewStoreError> {
            (**self).find_for_post(o, p).await
        }
        async fn revoke(&self, o: Uuid, p: Uuid) -> Result<(), DraftPreviewStoreError> {
            (**self).revoke(o, p).await
        }
        async fn find_live_by_token(
            &self,
            t: &str,
            n: DateTime<Utc>,
        ) -> Result<Option<LivePreview>, DraftPreviewStoreError> {
            (**self).find_live_by_token(t, n).await
        }
    }

    // ------------------------------------------------------------------
    // Minting and renewing
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn sharing_a_draft_mints_a_link_that_lasts_the_full_ttl() {
        let store = Arc::new(FakeStore::default());
        let before = Utc::now() + Duration::days(DRAFT_PREVIEW_TTL_DAYS) - Duration::minutes(1);

        let state = ShareDraftService::new(Arc::clone(&store))
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap();

        assert!(!state.expired);
        assert!(
            state.expires_at > before,
            "the TTL must be the full 14 days"
        );
        assert_eq!(state.token.len(), 64, "32 random bytes, hex encoded");
    }

    /// The amendment that separates this from a link that dies silently:
    /// renewing extends the existing link rather than minting a new one, so a
    /// reviewer's bookmark survives exactly when the author is trying to keep
    /// it alive.
    #[tokio::test]
    async fn renewing_keeps_the_same_link() {
        let store = Arc::new(FakeStore::default());
        let service = ShareDraftService::new(Arc::clone(&store));
        let owner = UserId::from(Uuid::new_v4());
        let post = Uuid::new_v4();

        let first = service.execute(owner, post).await.unwrap();
        let renewed = service.execute(owner, post).await.unwrap();

        assert_eq!(first.token, renewed.token, "the link must not change");
        assert_eq!(first.created_at, renewed.created_at);
        assert!(renewed.expires_at >= first.expires_at);
    }

    #[tokio::test]
    async fn sharing_another_authors_post_is_not_found() {
        let store = Arc::new(FakeStore {
            missing_post: true,
            ..Default::default()
        });

        let err = ShareDraftService::new(store)
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap_err();

        assert!(matches!(err, DraftPreviewError::PostNotFound));
    }

    // ------------------------------------------------------------------
    // The author's sharing panel
    // ------------------------------------------------------------------

    /// An expired link is reported, not hidden. The author has to be able to
    /// see that it lapsed — invisible expiry is the failure mode this design
    /// exists to avoid.
    #[tokio::test]
    async fn an_expired_link_is_reported_as_expired_not_missing() {
        let store = Arc::new(FakeStore::default());
        *store.row.lock().unwrap() =
            Some(a_preview(Uuid::new_v4(), Utc::now() - Duration::hours(1)));

        let state = GetDraftPreviewService::new(Arc::clone(&store))
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap();

        assert!(state.expired);
        assert_eq!(
            state.token, "tok",
            "the link is still shown, so it can be renewed"
        );
    }

    #[tokio::test]
    async fn a_post_that_was_never_shared_reports_not_shared() {
        let store = Arc::new(FakeStore::default());

        let err = GetDraftPreviewService::new(store)
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap_err();

        assert!(matches!(err, DraftPreviewError::NotShared));
    }

    #[tokio::test]
    async fn revoking_stops_the_link_working() {
        let store = Arc::new(FakeStore::default());
        *store.row.lock().unwrap() =
            Some(a_preview(Uuid::new_v4(), Utc::now() + Duration::days(1)));

        RevokeDraftPreviewService::new(Arc::clone(&store))
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap();

        let err = reader(store, None).execute("tok").await.unwrap_err();
        assert!(matches!(err, DraftPreviewError::PostNotFound));
    }

    // ------------------------------------------------------------------
    // The reader's side
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn a_live_link_serves_the_draft() {
        let store = Arc::new(FakeStore::default());
        *store.row.lock().unwrap() =
            Some(a_preview(Uuid::new_v4(), Utc::now() + Duration::days(3)));

        let resolved = reader(store, None).execute("tok").await.unwrap();

        assert!(matches!(resolved, PreviewResolution::Draft(_)));
    }

    /// A post scheduled for the future is not yet readable publicly, so the
    /// preview must keep working rather than redirecting to a page that 404s.
    #[tokio::test]
    async fn a_scheduled_post_still_previews() {
        let store = Arc::new(FakeStore::default());
        *store.row.lock().unwrap() =
            Some(a_preview(Uuid::new_v4(), Utc::now() + Duration::days(3)));

        let resolved = reader(store, Some(Utc::now() + Duration::days(2)))
            .execute("tok")
            .await
            .unwrap();

        assert!(matches!(resolved, PreviewResolution::Draft(_)));
    }

    /// The other amendment. Telling a reviewer "expired" for a post the whole
    /// world can now read is the worst moment this feature could produce, so a
    /// published post redirects to its public page instead.
    #[tokio::test]
    async fn a_published_post_redirects_to_its_public_page() {
        let store = Arc::new(FakeStore::default());
        *store.row.lock().unwrap() =
            Some(a_preview(Uuid::new_v4(), Utc::now() + Duration::days(3)));

        let resolved = reader(store, Some(Utc::now() - Duration::hours(1)))
            .execute("tok")
            .await
            .unwrap();

        match resolved {
            PreviewResolution::Published { username, slug } => {
                assert_eq!(username, "janedoe");
                assert_eq!(slug, "draft-slug");
            }
            other => panic!("expected a redirect, got {other:?}"),
        }
    }

    /// Expired, revoked and never-existed must be one answer. Anything else
    /// lets the holder of a dead link learn whether the draft still exists.
    #[tokio::test]
    async fn an_expired_token_is_indistinguishable_from_an_unknown_one() {
        let expired = Arc::new(FakeStore::default());
        *expired.row.lock().unwrap() =
            Some(a_preview(Uuid::new_v4(), Utc::now() - Duration::minutes(1)));

        let from_expired = reader(expired, None).execute("tok").await.unwrap_err();
        let from_unknown = reader(Arc::new(FakeStore::default()), None)
            .execute("never-minted")
            .await
            .unwrap_err();

        assert!(matches!(from_expired, DraftPreviewError::PostNotFound));
        assert!(matches!(from_unknown, DraftPreviewError::PostNotFound));
    }

    /// Two links must never collide, and must not be derivable from the post
    /// id — which appears in console URLs.
    #[tokio::test]
    async fn tokens_are_unpredictable() {
        let mut seen = std::collections::HashSet::new();
        for _ in 0..100 {
            assert!(seen.insert(mint_token()), "minted a duplicate token");
        }
    }

    // ------------------------------------------------------------------
    // Preview images
    // ------------------------------------------------------------------

    /// Answers only for the post it was told about; anything else is `None`,
    /// standing in for the attachment join the real adapter performs.
    struct FakeMedia {
        post_id: Uuid,
    }

    #[async_trait]
    impl PreviewMediaResolver for FakeMedia {
        async fn resolve(
            &self,
            post_id: Uuid,
            media_id: Uuid,
            size: &str,
        ) -> Result<Option<String>, String> {
            if post_id != self.post_id {
                return Ok(None);
            }
            Ok(Some(format!("https://signed.example/{media_id}/{size}")))
        }
    }

    fn media_reader(
        store: Arc<FakeStore>,
        attached_to: Uuid,
    ) -> ReadPreviewMediaService<Arc<FakeStore>> {
        ReadPreviewMediaService::new(
            store,
            Arc::new(FakeMedia {
                post_id: attached_to,
            }),
        )
    }

    #[tokio::test]
    async fn a_live_token_resolves_its_own_images() {
        let post_id = Uuid::new_v4();
        let store = Arc::new(FakeStore::default());
        *store.row.lock().unwrap() = Some(a_preview(post_id, Utc::now() + Duration::days(3)));

        let url = media_reader(store, post_id)
            .execute("tok", Uuid::new_v4(), "thumbnail")
            .await
            .unwrap();

        assert!(url.starts_with("https://signed.example/"));
    }

    /// The scoping that matters: a token opens one draft, not the media table.
    /// Without the post_id check it would resolve any media id in the system.
    #[tokio::test]
    async fn a_token_cannot_reach_another_posts_media() {
        let this_post = Uuid::new_v4();
        let some_other_post = Uuid::new_v4();
        let store = Arc::new(FakeStore::default());
        *store.row.lock().unwrap() = Some(a_preview(this_post, Utc::now() + Duration::days(3)));

        let err = media_reader(store, some_other_post)
            .execute("tok", Uuid::new_v4(), "thumbnail")
            .await
            .unwrap_err();

        assert!(matches!(err, DraftPreviewError::PostNotFound));
    }

    /// Revoking has to stop the pictures, not just the prose.
    #[tokio::test]
    async fn a_revoked_token_stops_resolving_images() {
        let post_id = Uuid::new_v4();
        let store = Arc::new(FakeStore::default());

        let err = media_reader(store, post_id)
            .execute("tok", Uuid::new_v4(), "thumbnail")
            .await
            .unwrap_err();

        assert!(matches!(err, DraftPreviewError::PostNotFound));
    }

    #[tokio::test]
    async fn an_expired_token_stops_resolving_images() {
        let post_id = Uuid::new_v4();
        let store = Arc::new(FakeStore::default());
        *store.row.lock().unwrap() = Some(a_preview(post_id, Utc::now() - Duration::minutes(1)));

        let err = media_reader(store, post_id)
            .execute("tok", Uuid::new_v4(), "thumbnail")
            .await
            .unwrap_err();

        assert!(matches!(err, DraftPreviewError::PostNotFound));
    }
}
