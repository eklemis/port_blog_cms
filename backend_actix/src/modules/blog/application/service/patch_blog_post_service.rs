use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::blog::application::ports::incoming::use_cases::{
    PatchBlogPostError, PatchBlogPostUseCase,
};
use crate::blog::application::ports::outgoing::{
    BlogPatchField, BlogPostRepository, BlogPostRepositoryError, PatchBlogPostData,
};
use crate::blog::domain::entities::BlogPost;

pub struct PatchBlogPostService<R>
where
    R: BlogPostRepository,
{
    repository: R,
}

impl<R> PatchBlogPostService<R>
where
    R: BlogPostRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    /// Same rules as creation. A patched slug still lands in a URL and still
    /// hits the per-author unique index, so it cannot be looser.
    fn validate_slug(slug: &str) -> Result<String, PatchBlogPostError> {
        let trimmed = slug.trim().to_lowercase();

        if trimmed.is_empty() {
            return Err(PatchBlogPostError::InvalidSlug(
                "Slug cannot be empty".to_string(),
            ));
        }
        if trimmed.len() > 200 {
            return Err(PatchBlogPostError::InvalidSlug(
                "Slug must not exceed 200 characters".to_string(),
            ));
        }
        if !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err(PatchBlogPostError::InvalidSlug(
                "Slug may contain only letters, numbers, and hyphens".to_string(),
            ));
        }

        Ok(trimmed)
    }
}

#[async_trait]
impl<R> PatchBlogPostUseCase for PatchBlogPostService<R>
where
    R: BlogPostRepository + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        post_id: Uuid,
        mut data: PatchBlogPostData,
    ) -> Result<BlogPost, PatchBlogPostError> {
        // The repository's patch is not owner-scoped — it takes a post id — so
        // ownership is established here before anything is written.
        let existing = self
            .repository
            .fetch_by_id(post_id)
            .await
            .map_err(|e| match e {
                BlogPostRepositoryError::NotFound => PatchBlogPostError::NotFound,
                BlogPostRepositoryError::SlugAlreadyExists => {
                    PatchBlogPostError::SlugAlreadyExists
                }
                BlogPostRepositoryError::DatabaseError(m) => {
                    PatchBlogPostError::RepositoryError(m)
                }
            })?
            .ok_or(PatchBlogPostError::NotFound)?;

        if existing.user_id != owner.value() {
            return Err(PatchBlogPostError::Unauthorized);
        }

        // Normalise a supplied slug before it reaches the adapter, so the
        // validation error is raised here rather than as a constraint failure.
        if let BlogPatchField::Value(slug) = &data.slug {
            data.slug = BlogPatchField::Value(Self::validate_slug(slug)?);
        }

        // A slug is required, so clearing it is not a meaningful request.
        if matches!(data.slug, BlogPatchField::Null) {
            return Err(PatchBlogPostError::InvalidSlug(
                "Slug cannot be cleared".to_string(),
            ));
        }

        self.repository
            .patch(post_id, data)
            .await
            .map_err(|e| match e {
                BlogPostRepositoryError::NotFound => PatchBlogPostError::NotFound,
                BlogPostRepositoryError::SlugAlreadyExists => {
                    PatchBlogPostError::SlugAlreadyExists
                }
                BlogPostRepositoryError::DatabaseError(m) => {
                    PatchBlogPostError::RepositoryError(m)
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blog::application::ports::outgoing::CreateBlogPostData;
    use chrono::Utc;
    use std::sync::Mutex;

    fn a_post(user_id: Uuid) -> BlogPost {
        let now = Utc::now();
        BlogPost {
            id: Uuid::new_v4(),
            user_id,
            title: "Hello".into(),
            slug: "hello".into(),
            excerpt: None,
            content: "body".into(),
            published_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    struct MockRepo {
        existing: Option<BlogPost>,
        patch_result: Result<BlogPost, BlogPostRepositoryError>,
        seen: Mutex<Option<PatchBlogPostData>>,
    }

    #[async_trait]
    impl BlogPostRepository for MockRepo {
        async fn create(
            &self,
            _d: CreateBlogPostData,
        ) -> Result<BlogPost, BlogPostRepositoryError> {
            unimplemented!()
        }
        async fn fetch_by_id(
            &self,
            _id: Uuid,
        ) -> Result<Option<BlogPost>, BlogPostRepositoryError> {
            Ok(self.existing.clone())
        }
        async fn patch(
            &self,
            _id: Uuid,
            data: PatchBlogPostData,
        ) -> Result<BlogPost, BlogPostRepositoryError> {
            *self.seen.lock().unwrap() = Some(data);
            self.patch_result.clone()
        }
    }

    fn service(existing: Option<BlogPost>) -> PatchBlogPostService<MockRepo> {
        let patched = existing.clone().unwrap_or_else(|| a_post(Uuid::new_v4()));
        PatchBlogPostService::new(MockRepo {
            existing,
            patch_result: Ok(patched),
            seen: Mutex::new(None),
        })
    }

    #[tokio::test]
    async fn patches_a_post_the_caller_owns() {
        let user_id = Uuid::new_v4();
        let svc = service(Some(a_post(user_id)));

        assert!(svc
            .execute(
                UserId::from(user_id),
                Uuid::new_v4(),
                PatchBlogPostData::default()
            )
            .await
            .is_ok());
    }

    /// The repository's patch takes only a post id, so without this check any
    /// authenticated user could edit any post.
    #[tokio::test]
    async fn refuses_to_patch_another_users_post() {
        let svc = service(Some(a_post(Uuid::new_v4())));

        let err = svc
            .execute(
                UserId::from(Uuid::new_v4()),
                Uuid::new_v4(),
                PatchBlogPostData::default(),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, PatchBlogPostError::Unauthorized));
    }

    #[tokio::test]
    async fn reports_not_found_for_a_missing_post() {
        let svc = service(None);

        let err = svc
            .execute(
                UserId::from(Uuid::new_v4()),
                Uuid::new_v4(),
                PatchBlogPostData::default(),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, PatchBlogPostError::NotFound));
    }

    #[tokio::test]
    async fn normalises_a_supplied_slug() {
        let user_id = Uuid::new_v4();
        let svc = service(Some(a_post(user_id)));

        svc.execute(
            UserId::from(user_id),
            Uuid::new_v4(),
            PatchBlogPostData {
                slug: BlogPatchField::Value("  My-NEW-Slug  ".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let seen = svc.repository.seen.lock().unwrap().clone().unwrap();
        assert_eq!(seen.slug.as_value().unwrap(), "my-new-slug");
    }

    #[tokio::test]
    async fn rejects_an_invalid_slug() {
        let user_id = Uuid::new_v4();
        let svc = service(Some(a_post(user_id)));

        let err = svc
            .execute(
                UserId::from(user_id),
                Uuid::new_v4(),
                PatchBlogPostData {
                    slug: BlogPatchField::Value("not a slug".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, PatchBlogPostError::InvalidSlug(_)));
    }

    /// `Null` means "clear this column", which is meaningful for excerpt and
    /// published_at but not for a slug the URL depends on.
    #[tokio::test]
    async fn rejects_clearing_the_slug() {
        let user_id = Uuid::new_v4();
        let svc = service(Some(a_post(user_id)));

        let err = svc
            .execute(
                UserId::from(user_id),
                Uuid::new_v4(),
                PatchBlogPostData {
                    slug: BlogPatchField::Null,
                    ..Default::default()
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, PatchBlogPostError::InvalidSlug(_)));
    }

    /// Unpublishing is a Null on published_at, and must pass through untouched.
    #[tokio::test]
    async fn allows_unpublishing_via_a_null_publish_date() {
        let user_id = Uuid::new_v4();
        let svc = service(Some(a_post(user_id)));

        svc.execute(
            UserId::from(user_id),
            Uuid::new_v4(),
            PatchBlogPostData {
                published_at: BlogPatchField::Null,
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let seen = svc.repository.seen.lock().unwrap().clone().unwrap();
        assert!(matches!(seen.published_at, BlogPatchField::Null));
    }

    /// The repository is consulted twice — once to establish ownership, once to
    /// write — and each call maps its errors independently. Both mappings are
    /// exercised here, since a mistake in either turns a 404 or 409 into a 500.
    #[tokio::test]
    async fn errors_from_the_ownership_read_are_mapped() {
        struct FailingFetch(BlogPostRepositoryError);

        #[async_trait]
        impl BlogPostRepository for FailingFetch {
            async fn create(
                &self,
                _d: CreateBlogPostData,
            ) -> Result<BlogPost, BlogPostRepositoryError> {
                unimplemented!()
            }
            async fn fetch_by_id(
                &self,
                _id: Uuid,
            ) -> Result<Option<BlogPost>, BlogPostRepositoryError> {
                Err(self.0.clone())
            }
            async fn patch(
                &self,
                _id: Uuid,
                _d: PatchBlogPostData,
            ) -> Result<BlogPost, BlogPostRepositoryError> {
                unimplemented!()
            }
        }

        let cases = [
            (BlogPostRepositoryError::NotFound, "notfound"),
            (BlogPostRepositoryError::SlugAlreadyExists, "slug"),
            (BlogPostRepositoryError::DatabaseError("db".into()), "db"),
        ];

        for (err, kind) in cases {
            let svc = PatchBlogPostService::new(FailingFetch(err));
            let got = svc
                .execute(
                    UserId::from(Uuid::new_v4()),
                    Uuid::new_v4(),
                    PatchBlogPostData::default(),
                )
                .await
                .unwrap_err();

            match kind {
                "notfound" => assert!(matches!(got, PatchBlogPostError::NotFound)),
                "slug" => assert!(matches!(got, PatchBlogPostError::SlugAlreadyExists)),
                _ => assert!(matches!(got, PatchBlogPostError::RepositoryError(m) if m == "db")),
            }
        }
    }

    /// A slug collision can only be detected by the database, on the write —
    /// the service cannot know another post holds the slug until the unique
    /// index rejects it. So this arm is the one that produces the 409.
    #[tokio::test]
    async fn errors_from_the_write_are_mapped() {
        let user_id = Uuid::new_v4();

        for (err, expect_slug) in [
            (BlogPostRepositoryError::SlugAlreadyExists, true),
            (BlogPostRepositoryError::NotFound, false),
            (BlogPostRepositoryError::DatabaseError("db".into()), false),
        ] {
            let svc = PatchBlogPostService::new(MockRepo {
                existing: Some(a_post(user_id)),
                patch_result: Err(err),
                seen: Mutex::new(None),
            });

            let got = svc
                .execute(
                    UserId::from(user_id),
                    Uuid::new_v4(),
                    PatchBlogPostData::default(),
                )
                .await
                .unwrap_err();

            if expect_slug {
                assert!(matches!(got, PatchBlogPostError::SlugAlreadyExists));
            } else {
                assert!(!matches!(got, PatchBlogPostError::SlugAlreadyExists));
            }
        }
    }

    /// Slug validation matches creation exactly: a patched slug still lands in
    /// a URL and still meets the same unique index.
    #[tokio::test]
    async fn a_patched_slug_faces_the_same_rules_as_a_created_one() {
        let user_id = Uuid::new_v4();

        for bad in ["", "   ", "has space", "has/slash", "héllo", &"a".repeat(201)] {
            let svc = service(Some(a_post(user_id)));
            let err = svc
                .execute(
                    UserId::from(user_id),
                    Uuid::new_v4(),
                    PatchBlogPostData {
                        slug: BlogPatchField::Value(bad.to_string()),
                        ..Default::default()
                    },
                )
                .await
                .unwrap_err();

            assert!(
                matches!(err, PatchBlogPostError::InvalidSlug(_)),
                "{bad:?} should be rejected"
            );
        }
    }
}
