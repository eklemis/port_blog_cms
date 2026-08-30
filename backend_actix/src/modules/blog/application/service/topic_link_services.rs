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
