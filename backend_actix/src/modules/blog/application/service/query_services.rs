//! Read-side services.
//!
//! These are thin: the query adapter owns filtering and pagination, so each
//! service exists to map an outgoing error onto the error type its endpoint
//! speaks, and — for the public variants — to pick the published-only query.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::blog::application::ports::incoming::use_cases::{
    GetBlogPostError, GetBlogPostTopicsUseCase, GetBlogPostsError, GetBlogPostsUseCase,
    GetPublicBlogPostUseCase, GetPublicBlogPostsUseCase, GetSingleBlogPostUseCase,
};
use crate::blog::application::ports::outgoing::{
    BlogPageRequest, BlogPageResult, BlogPostCard, BlogPostListFilter, BlogPostQuery,
    BlogPostQueryError, BlogPostSort, BlogPostView,
};
use crate::blog::domain::entities::BlogPostTopic;

macro_rules! query_service {
    ($name:ident) => {
        /// A read-side blog service. Thin: the query adapter owns filtering and
        /// pagination, so this maps the outgoing error onto the endpoint's.
        pub struct $name<Q>
        where
            Q: BlogPostQuery,
        {
            query: Q,
        }

        impl<Q> $name<Q>
        where
            Q: BlogPostQuery,
        {
            /// Builds it from the ports it depends on.
            pub fn new(query: Q) -> Self {
                Self { query }
            }
        }
    };
}

query_service!(GetBlogPostsService);
query_service!(GetPublicBlogPostsService);
query_service!(GetSingleBlogPostService);
query_service!(GetPublicBlogPostService);
query_service!(GetBlogPostTopicsService);

fn list_err(e: BlogPostQueryError) -> GetBlogPostsError {
    match e {
        BlogPostQueryError::NotFound => {
            GetBlogPostsError::QueryFailed("post not found".to_string())
        }
        BlogPostQueryError::DatabaseError(m) => GetBlogPostsError::QueryFailed(m),
    }
}

#[async_trait]
impl<Q> GetBlogPostsUseCase for GetBlogPostsService<Q>
where
    Q: BlogPostQuery + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        filter: BlogPostListFilter,
        sort: BlogPostSort,
        page: BlogPageRequest,
    ) -> Result<BlogPageResult<BlogPostCard>, GetBlogPostsError> {
        self.query
            .list_by_owner(owner, filter, sort, page)
            .await
            .map_err(list_err)
    }
}

#[async_trait]
impl<Q> GetPublicBlogPostsUseCase for GetPublicBlogPostsService<Q>
where
    Q: BlogPostQuery + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        filter: BlogPostListFilter,
        sort: BlogPostSort,
        page: BlogPageRequest,
    ) -> Result<BlogPageResult<BlogPostCard>, GetBlogPostsError> {
        // Deliberately the published-only query. The filter still arrives from
        // the query string, and the adapter ignores its `published` field here.
        self.query
            .list_published(owner, filter, sort, page)
            .await
            .map_err(list_err)
    }
}

#[async_trait]
impl<Q> GetSingleBlogPostUseCase for GetSingleBlogPostService<Q>
where
    Q: BlogPostQuery + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        post_id: Uuid,
    ) -> Result<BlogPostView, GetBlogPostError> {
        self.query
            .get_by_id(owner, post_id)
            .await
            .map_err(GetBlogPostError::from)
    }
}

#[async_trait]
impl<Q> GetPublicBlogPostUseCase for GetPublicBlogPostService<Q>
where
    Q: BlogPostQuery + Send + Sync,
{
    async fn execute(&self, owner: UserId, slug: &str) -> Result<BlogPostView, GetBlogPostError> {
        self.query
            .get_published_by_slug(owner, slug)
            .await
            .map_err(GetBlogPostError::from)
    }
}

#[async_trait]
impl<Q> GetBlogPostTopicsUseCase for GetBlogPostTopicsService<Q>
where
    Q: BlogPostQuery + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        post_id: Uuid,
    ) -> Result<Vec<BlogPostTopic>, GetBlogPostError> {
        // Resolve the post first so a caller cannot read the topics of a post
        // they do not own, or of one that is archived.
        self.query
            .get_by_id(owner, post_id)
            .await
            .map_err(GetBlogPostError::from)?;

        self.query
            .get_topics(post_id)
            .await
            .map_err(GetBlogPostError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blog::domain::entities::BlogPost;
    use chrono::Utc;
    use std::sync::Mutex;

    /// Records which query method the service reached for. These services are
    /// thin, so the behaviour worth pinning is the routing decision — above all
    /// that the public listing uses `list_published` and never `list_by_owner`.
    #[derive(Default)]
    struct SpyQuery {
        called: Mutex<Vec<&'static str>>,
        fail: bool,
        not_found: bool,
    }

    impl SpyQuery {
        fn record(&self, what: &'static str) {
            self.called.lock().unwrap().push(what);
        }
        fn err<T>(&self) -> Option<Result<T, BlogPostQueryError>> {
            if self.fail {
                Some(Err(BlogPostQueryError::DatabaseError("db down".into())))
            } else if self.not_found {
                Some(Err(BlogPostQueryError::NotFound))
            } else {
                None
            }
        }
    }

    fn a_post() -> BlogPost {
        let now = Utc::now();
        BlogPost {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            title: "t".into(),
            slug: "t".into(),
            excerpt: None,
            content: "c".into(),
            published_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn empty_page() -> BlogPageResult<BlogPostCard> {
        BlogPageResult {
            items: vec![],
            page: 1,
            per_page: 10,
            total: 0,
        }
    }

    #[async_trait]
    impl BlogPostQuery for SpyQuery {
        async fn list_by_owner(
            &self,
            _o: UserId,
            _f: BlogPostListFilter,
            _s: BlogPostSort,
            _p: BlogPageRequest,
        ) -> Result<BlogPageResult<BlogPostCard>, BlogPostQueryError> {
            self.record("list_by_owner");
            self.err().unwrap_or_else(|| Ok(empty_page()))
        }
        async fn list_published(
            &self,
            _o: UserId,
            _f: BlogPostListFilter,
            _s: BlogPostSort,
            _p: BlogPageRequest,
        ) -> Result<BlogPageResult<BlogPostCard>, BlogPostQueryError> {
            self.record("list_published");
            self.err().unwrap_or_else(|| Ok(empty_page()))
        }
        async fn get_by_id(
            &self,
            _o: UserId,
            _p: Uuid,
        ) -> Result<BlogPostView, BlogPostQueryError> {
            self.record("get_by_id");
            self.err().unwrap_or_else(|| {
                Ok(BlogPostView {
                    post: a_post(),
                    topics: vec![],
                })
            })
        }
        async fn get_published_by_slug(
            &self,
            _o: UserId,
            _s: &str,
        ) -> Result<BlogPostView, BlogPostQueryError> {
            self.record("get_published_by_slug");
            self.err().unwrap_or_else(|| {
                Ok(BlogPostView {
                    post: a_post(),
                    topics: vec![],
                })
            })
        }
        async fn get_topics(&self, _p: Uuid) -> Result<Vec<BlogPostTopic>, BlogPostQueryError> {
            self.record("get_topics");
            self.err().unwrap_or_else(|| {
                Ok(vec![BlogPostTopic {
                    id: Uuid::new_v4(),
                    title: "Rust".into(),
                    description: "d".into(),
                }])
            })
        }
    }

    fn owner() -> UserId {
        UserId::from(Uuid::new_v4())
    }

    #[tokio::test]
    async fn owner_listing_uses_the_owner_query() {
        let svc = GetBlogPostsService::new(SpyQuery::default());
        svc.execute(
            owner(),
            BlogPostListFilter::default(),
            BlogPostSort::Newest,
            BlogPageRequest::default(),
        )
        .await
        .unwrap();
        assert_eq!(
            svc.query.called.lock().unwrap().as_slice(),
            ["list_by_owner"]
        );
    }

    /// The whole reason this is a separate service: it must never reach
    /// list_by_owner, which would expose drafts on a public endpoint.
    #[tokio::test]
    async fn public_listing_uses_the_published_only_query() {
        let svc = GetPublicBlogPostsService::new(SpyQuery::default());
        svc.execute(
            owner(),
            BlogPostListFilter {
                published: Some(false),
                ..Default::default()
            },
            BlogPostSort::Newest,
            BlogPageRequest::default(),
        )
        .await
        .unwrap();
        assert_eq!(
            svc.query.called.lock().unwrap().as_slice(),
            ["list_published"]
        );
    }

    #[tokio::test]
    async fn public_single_uses_the_slug_query() {
        let svc = GetPublicBlogPostService::new(SpyQuery::default());
        svc.execute(owner(), "hello").await.unwrap();
        assert_eq!(
            svc.query.called.lock().unwrap().as_slice(),
            ["get_published_by_slug"]
        );
    }

    #[tokio::test]
    async fn single_post_uses_the_owner_scoped_query() {
        let svc = GetSingleBlogPostService::new(SpyQuery::default());
        svc.execute(owner(), Uuid::new_v4()).await.unwrap();
        assert_eq!(svc.query.called.lock().unwrap().as_slice(), ["get_by_id"]);
    }

    /// Topics are read only after the post resolves through the owner-scoped
    /// query, so a caller cannot read the topics of someone else's post.
    #[tokio::test]
    async fn topics_are_gated_behind_an_ownership_check() {
        let svc = GetBlogPostTopicsService::new(SpyQuery::default());
        svc.execute(owner(), Uuid::new_v4()).await.unwrap();
        assert_eq!(
            svc.query.called.lock().unwrap().as_slice(),
            ["get_by_id", "get_topics"]
        );
    }

    #[tokio::test]
    async fn topics_stop_at_the_ownership_check_when_the_post_is_not_found() {
        let svc = GetBlogPostTopicsService::new(SpyQuery {
            not_found: true,
            ..Default::default()
        });

        let err = svc.execute(owner(), Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, GetBlogPostError::NotFound));
        // get_topics is never reached.
        assert_eq!(svc.query.called.lock().unwrap().as_slice(), ["get_by_id"]);
    }

    #[tokio::test]
    async fn listing_errors_map_to_query_failed() {
        let svc = GetBlogPostsService::new(SpyQuery {
            fail: true,
            ..Default::default()
        });
        let err = svc
            .execute(
                owner(),
                BlogPostListFilter::default(),
                BlogPostSort::Newest,
                BlogPageRequest::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, GetBlogPostsError::QueryFailed(m) if m.contains("db down")));
    }

    /// NotFound has no meaning for a listing, so it is folded into QueryFailed
    /// rather than silently returning an empty page.
    #[tokio::test]
    async fn a_not_found_from_a_listing_is_still_a_failure() {
        let svc = GetBlogPostsService::new(SpyQuery {
            not_found: true,
            ..Default::default()
        });
        let err = svc
            .execute(
                owner(),
                BlogPostListFilter::default(),
                BlogPostSort::Newest,
                BlogPageRequest::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, GetBlogPostsError::QueryFailed(_)));
    }

    #[tokio::test]
    async fn single_post_errors_map_to_their_own_variants() {
        let nf = GetSingleBlogPostService::new(SpyQuery {
            not_found: true,
            ..Default::default()
        });
        assert!(matches!(
            nf.execute(owner(), Uuid::new_v4()).await.unwrap_err(),
            GetBlogPostError::NotFound
        ));

        let db = GetSingleBlogPostService::new(SpyQuery {
            fail: true,
            ..Default::default()
        });
        assert!(matches!(
            db.execute(owner(), Uuid::new_v4()).await.unwrap_err(),
            GetBlogPostError::QueryFailed(_)
        ));
    }
}
