//! Attach, detach, and clear services for post/topic links.
//!
//! The repository already scopes every operation by owner and distinguishes a
//! missing post from a missing topic, so these map its error onto the incoming
//! type and nothing more.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::blog::application::ports::incoming::use_cases::{
    AttachBlogPostTopicUseCase, BlogPostTopicError, ClearBlogPostTopicsUseCase,
    DetachBlogPostTopicUseCase,
};
use crate::blog::application::ports::outgoing::BlogPostTopicRepository;

macro_rules! topic_link_service {
    ($name:ident) => {
        pub struct $name<R>
        where
            R: BlogPostTopicRepository,
        {
            repository: R,
        }

        impl<R> $name<R>
        where
            R: BlogPostTopicRepository,
        {
            pub fn new(repository: R) -> Self {
                Self { repository }
            }
        }
    };
}

topic_link_service!(AttachBlogPostTopicService);
topic_link_service!(DetachBlogPostTopicService);
topic_link_service!(ClearBlogPostTopicsService);

#[async_trait]
impl<R> AttachBlogPostTopicUseCase for AttachBlogPostTopicService<R>
where
    R: BlogPostTopicRepository + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        post_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), BlogPostTopicError> {
        self.repository
            .attach(owner, post_id, topic_id)
            .await
            .map_err(BlogPostTopicError::from)
    }
}

#[async_trait]
impl<R> DetachBlogPostTopicUseCase for DetachBlogPostTopicService<R>
where
    R: BlogPostTopicRepository + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        post_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), BlogPostTopicError> {
        self.repository
            .detach(owner, post_id, topic_id)
            .await
            .map_err(BlogPostTopicError::from)
    }
}

#[async_trait]
impl<R> ClearBlogPostTopicsUseCase for ClearBlogPostTopicsService<R>
where
    R: BlogPostTopicRepository + Send + Sync,
{
    async fn execute(&self, owner: UserId, post_id: Uuid) -> Result<(), BlogPostTopicError> {
        self.repository
            .clear(owner, post_id)
            .await
            .map_err(BlogPostTopicError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blog::application::ports::outgoing::BlogPostTopicRepositoryError;
    use std::sync::Mutex;

    #[derive(Default)]
    struct SpyRepo {
        called: Mutex<Vec<&'static str>>,
        result: Option<BlogPostTopicRepositoryError>,
    }

    impl SpyRepo {
        fn failing(e: BlogPostTopicRepositoryError) -> Self {
            Self {
                called: Mutex::new(vec![]),
                result: Some(e),
            }
        }
        fn out(&self, what: &'static str) -> Result<(), BlogPostTopicRepositoryError> {
            self.called.lock().unwrap().push(what);
            match &self.result {
                Some(e) => Err(e.clone()),
                None => Ok(()),
            }
        }
    }

    #[async_trait]
    impl BlogPostTopicRepository for SpyRepo {
        async fn attach(
            &self,
            _o: UserId,
            _p: Uuid,
            _t: Uuid,
        ) -> Result<(), BlogPostTopicRepositoryError> {
            self.out("attach")
        }
        async fn detach(
            &self,
            _o: UserId,
            _p: Uuid,
            _t: Uuid,
        ) -> Result<(), BlogPostTopicRepositoryError> {
            self.out("detach")
        }
        async fn clear(&self, _o: UserId, _p: Uuid) -> Result<(), BlogPostTopicRepositoryError> {
            self.out("clear")
        }
    }

    fn owner() -> UserId {
        UserId::from(Uuid::new_v4())
    }

    /// attach and detach take the same arguments, so a service wired to the
    /// wrong one would silently do the opposite of what was asked.
    #[tokio::test]
    async fn each_service_calls_its_own_repository_operation() {
        let a = AttachBlogPostTopicService::new(SpyRepo::default());
        a.execute(owner(), Uuid::new_v4(), Uuid::new_v4()).await.unwrap();
        assert_eq!(a.repository.called.lock().unwrap().as_slice(), ["attach"]);

        let d = DetachBlogPostTopicService::new(SpyRepo::default());
        d.execute(owner(), Uuid::new_v4(), Uuid::new_v4()).await.unwrap();
        assert_eq!(d.repository.called.lock().unwrap().as_slice(), ["detach"]);

        let c = ClearBlogPostTopicsService::new(SpyRepo::default());
        c.execute(owner(), Uuid::new_v4()).await.unwrap();
        assert_eq!(c.repository.called.lock().unwrap().as_slice(), ["clear"]);
    }

    /// PostNotFound and TopicNotFound must stay distinguishable through the
    /// service, since the route reports different codes for each.
    #[tokio::test]
    async fn post_and_topic_not_found_stay_distinct() {
        let p = AttachBlogPostTopicService::new(SpyRepo::failing(
            BlogPostTopicRepositoryError::PostNotFound,
        ));
        assert!(matches!(
            p.execute(owner(), Uuid::new_v4(), Uuid::new_v4()).await.unwrap_err(),
            BlogPostTopicError::PostNotFound
        ));

        let t = AttachBlogPostTopicService::new(SpyRepo::failing(
            BlogPostTopicRepositoryError::TopicNotFound,
        ));
        assert!(matches!(
            t.execute(owner(), Uuid::new_v4(), Uuid::new_v4()).await.unwrap_err(),
            BlogPostTopicError::TopicNotFound
        ));
    }

    #[tokio::test]
    async fn database_errors_map_across_all_three() {
        let e = || BlogPostTopicRepositoryError::DatabaseError("db down".into());

        let a = AttachBlogPostTopicService::new(SpyRepo::failing(e()));
        assert!(matches!(
            a.execute(owner(), Uuid::new_v4(), Uuid::new_v4()).await.unwrap_err(),
            BlogPostTopicError::RepositoryError(_)
        ));

        let d = DetachBlogPostTopicService::new(SpyRepo::failing(e()));
        assert!(matches!(
            d.execute(owner(), Uuid::new_v4(), Uuid::new_v4()).await.unwrap_err(),
            BlogPostTopicError::RepositoryError(_)
        ));

        let c = ClearBlogPostTopicsService::new(SpyRepo::failing(e()));
        assert!(matches!(
            c.execute(owner(), Uuid::new_v4()).await.unwrap_err(),
            BlogPostTopicError::RepositoryError(_)
        ));
    }
}
