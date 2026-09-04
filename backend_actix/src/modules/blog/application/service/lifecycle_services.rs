//! Archive, restore, and hard-delete services.
//!
//! Each is a thin pass-through to the archiver, which is already owner-scoped
//! in SQL, so no separate ownership read is needed.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::blog::application::ports::incoming::use_cases::{
    ArchiveBlogPostError, ArchiveBlogPostUseCase, HardDeleteBlogPostUseCase,
    RestoreBlogPostUseCase, UnpublishBlogPostUseCase,
};
use crate::blog::application::ports::outgoing::BlogPostArchiver;

macro_rules! archiver_service {
    ($name:ident) => {
        /// An archive, restore or hard-delete service. A pass-through to the
        /// archiver, which is already owner-scoped in SQL.
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
            /// Builds it from the ports it depends on.
            pub fn new(archiver: A) -> Self {
                Self { archiver }
            }
        }
    };
}

archiver_service!(ArchiveBlogPostService);
archiver_service!(RestoreBlogPostService);
archiver_service!(HardDeleteBlogPostService);
archiver_service!(UnpublishBlogPostService);

#[async_trait]
impl<A> UnpublishBlogPostUseCase for UnpublishBlogPostService<A>
where
    A: BlogPostArchiver + Send + Sync,
{
    async fn execute(&self, owner: UserId, post_id: Uuid) -> Result<(), ArchiveBlogPostError> {
        self.archiver
            .unpublish(owner, post_id)
            .await
            .map_err(Into::into)
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blog::application::ports::outgoing::BlogPostArchiverError;
    use std::sync::Mutex;

    /// Records which archiver operation ran. These services are pass-throughs,
    /// so the behaviour worth pinning is that each reaches for the right one —
    /// archive calling hard_delete would be unrecoverable.
    #[derive(Default)]
    struct SpyArchiver {
        called: Mutex<Vec<&'static str>>,
        result: Option<BlogPostArchiverError>,
    }

    impl SpyArchiver {
        fn failing(e: BlogPostArchiverError) -> Self {
            Self {
                called: Mutex::new(vec![]),
                result: Some(e),
            }
        }
        fn out(&self, what: &'static str) -> Result<(), BlogPostArchiverError> {
            self.called.lock().unwrap().push(what);
            match &self.result {
                Some(e) => Err(e.clone()),
                None => Ok(()),
            }
        }
    }

    #[async_trait]
    impl BlogPostArchiver for SpyArchiver {
        async fn unpublish(
            &self,
            owner: UserId,
            post_id: Uuid,
        ) -> Result<(), BlogPostArchiverError> {
            self.soft_delete(owner, post_id).await
        }

        async fn soft_delete(&self, _o: UserId, _p: Uuid) -> Result<(), BlogPostArchiverError> {
            self.out("soft_delete")
        }
        async fn restore(&self, _o: UserId, _p: Uuid) -> Result<(), BlogPostArchiverError> {
            self.out("restore")
        }
        async fn hard_delete(&self, _o: UserId, _p: Uuid) -> Result<(), BlogPostArchiverError> {
            self.out("hard_delete")
        }
    }

    fn owner() -> UserId {
        UserId::from(Uuid::new_v4())
    }

    #[tokio::test]
    async fn each_service_calls_its_own_archiver_operation() {
        let a = ArchiveBlogPostService::new(SpyArchiver::default());
        a.execute(owner(), Uuid::new_v4()).await.unwrap();
        assert_eq!(
            a.archiver.called.lock().unwrap().as_slice(),
            ["soft_delete"]
        );

        let r = RestoreBlogPostService::new(SpyArchiver::default());
        r.execute(owner(), Uuid::new_v4()).await.unwrap();
        assert_eq!(r.archiver.called.lock().unwrap().as_slice(), ["restore"]);

        let h = HardDeleteBlogPostService::new(SpyArchiver::default());
        h.execute(owner(), Uuid::new_v4()).await.unwrap();
        assert_eq!(
            h.archiver.called.lock().unwrap().as_slice(),
            ["hard_delete"]
        );
    }

    #[tokio::test]
    async fn not_found_maps_across_all_three() {
        for svc in [0, 1, 2] {
            let spy = SpyArchiver::failing(BlogPostArchiverError::NotFound);
            let err = match svc {
                0 => {
                    ArchiveBlogPostService::new(spy)
                        .execute(owner(), Uuid::new_v4())
                        .await
                }
                1 => {
                    RestoreBlogPostService::new(spy)
                        .execute(owner(), Uuid::new_v4())
                        .await
                }
                _ => {
                    HardDeleteBlogPostService::new(spy)
                        .execute(owner(), Uuid::new_v4())
                        .await
                }
            }
            .unwrap_err();
            assert!(matches!(err, ArchiveBlogPostError::NotFound));
        }
    }

    #[tokio::test]
    async fn database_errors_map_across_all_three() {
        for svc in [0, 1, 2] {
            let spy = SpyArchiver::failing(BlogPostArchiverError::DatabaseError("db down".into()));
            let err = match svc {
                0 => {
                    ArchiveBlogPostService::new(spy)
                        .execute(owner(), Uuid::new_v4())
                        .await
                }
                1 => {
                    RestoreBlogPostService::new(spy)
                        .execute(owner(), Uuid::new_v4())
                        .await
                }
                _ => {
                    HardDeleteBlogPostService::new(spy)
                        .execute(owner(), Uuid::new_v4())
                        .await
                }
            }
            .unwrap_err();
            assert!(
                matches!(err, ArchiveBlogPostError::RepositoryError(m) if m.contains("db down"))
            );
        }
    }
}
