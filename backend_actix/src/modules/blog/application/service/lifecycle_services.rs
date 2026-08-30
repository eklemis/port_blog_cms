//! Archive, restore, and hard-delete services.
//!
//! Each is a thin pass-through to the archiver, which is already owner-scoped
//! in SQL, so no separate ownership read is needed.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::blog::application::ports::incoming::use_cases::{
    ArchiveBlogPostError, ArchiveBlogPostUseCase, HardDeleteBlogPostUseCase,
    RestoreBlogPostUseCase,
};
use crate::blog::application::ports::outgoing::BlogPostArchiver;

macro_rules! archiver_service {
    ($name:ident) => {
        pub struct $name<A>
        where
            A: BlogPostArchiver,
        {
            archiver: A,
        }

        impl<A> $name<A>
        where
            A: BlogPostArchiver,
        {
            pub fn new(archiver: A) -> Self {
                Self { archiver }
            }
        }
    };
}

archiver_service!(ArchiveBlogPostService);
archiver_service!(RestoreBlogPostService);
archiver_service!(HardDeleteBlogPostService);

#[async_trait]
impl<A> ArchiveBlogPostUseCase for ArchiveBlogPostService<A>
where
    A: BlogPostArchiver + Send + Sync,
{
    async fn execute(&self, owner: UserId, post_id: Uuid) -> Result<(), ArchiveBlogPostError> {
        self.archiver
            .soft_delete(owner, post_id)
            .await
            .map_err(ArchiveBlogPostError::from)
    }
}

#[async_trait]
impl<A> RestoreBlogPostUseCase for RestoreBlogPostService<A>
where
    A: BlogPostArchiver + Send + Sync,
{
    async fn execute(&self, owner: UserId, post_id: Uuid) -> Result<(), ArchiveBlogPostError> {
        self.archiver
            .restore(owner, post_id)
            .await
            .map_err(ArchiveBlogPostError::from)
    }
}

#[async_trait]
impl<A> HardDeleteBlogPostUseCase for HardDeleteBlogPostService<A>
where
    A: BlogPostArchiver + Send + Sync,
{
    async fn execute(&self, owner: UserId, post_id: Uuid) -> Result<(), ArchiveBlogPostError> {
        self.archiver
            .hard_delete(owner, post_id)
            .await
            .map_err(ArchiveBlogPostError::from)
    }
}
