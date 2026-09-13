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

/// Implements the corresponding use-case contract.
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
    /// Builds it from the ports it depends on.
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
                BlogPostRepositoryError::SlugAlreadyExists => PatchBlogPostError::SlugAlreadyExists,
                BlogPostRepositoryError::DatabaseError(m) => PatchBlogPostError::RepositoryError(m),
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

        // Publishing is the moment the body stops being private, so it is the
        // moment the body has to exist. The value compared is what the post
        // will hold after this patch: the incoming content when the same
        // request supplies it, and the stored content otherwise, since
        // publishing an already-written post sends no content at all.
        if matches!(data.published_at, BlogPatchField::Value(_)) {
            let will_be = match &data.content {
                BlogPatchField::Value(c) => c.as_str(),
                BlogPatchField::Null => "",
                BlogPatchField::Unset => existing.content.as_str(),
            };

            if will_be.trim().is_empty() {
                return Err(PatchBlogPostError::InvalidContent(
                    "A post cannot be published with an empty body".to_string(),
                ));
            }
        }

        self.repository
            .patch(post_id, data)
            .await
            .map_err(|e| match e {
                BlogPostRepositoryError::NotFound => PatchBlogPostError::NotFound,
                BlogPostRepositoryError::SlugAlreadyExists => PatchBlogPostError::SlugAlreadyExists,
                BlogPostRepositoryError::DatabaseError(m) => PatchBlogPostError::RepositoryError(m),
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
    /// The guard that moved here from creation. A post may sit blank for as
    /// long as its author likes; it may not go in front of a reader that way.
    #[tokio::test]
    async fn publishing_an_empty_post_is_refused() {
        let user_id = Uuid::new_v4();
        let mut blank = a_post(user_id);
        blank.content = "   ".into();

        let err = service(Some(blank))
            .execute(
                UserId::from(user_id),
                Uuid::new_v4(),
                PatchBlogPostData {
                    published_at: BlogPatchField::Value(Utc::now()),
                    ..Default::default()
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, PatchBlogPostError::InvalidContent(_)));
    }

    /// The body can arrive in the same request that publishes it, which is what
    /// "write it, then hit publish" looks like when the editor saves once.
    #[tokio::test]
    async fn publishing_with_a_body_in_the_same_patch_is_allowed() {
        let user_id = Uuid::new_v4();
        let mut blank = a_post(user_id);
        blank.content = "".into();

        assert!(service(Some(blank))
            .execute(
                UserId::from(user_id),
                Uuid::new_v4(),
                PatchBlogPostData {
                    published_at: BlogPatchField::Value(Utc::now()),
                    content: BlogPatchField::Value("Now it says something".into()),
                    ..Default::default()
                },
            )
            .await
            .is_ok());
    }

    /// Publishing an already-written post sends no content at all, so the
    /// stored body is what has to be checked.
    #[tokio::test]
    async fn publishing_an_already_written_post_sends_no_content() {
        let user_id = Uuid::new_v4();

        assert!(service(Some(a_post(user_id)))
            .execute(
                UserId::from(user_id),
                Uuid::new_v4(),
                PatchBlogPostData {
                    published_at: BlogPatchField::Value(Utc::now()),
                    ..Default::default()
                },
            )
            .await
            .is_ok());
    }

    /// Emptying the body of a post being published is still publishing an
    /// empty post, however it is spelled.
    #[tokio::test]
    async fn clearing_the_body_while_publishing_is_refused() {
        let user_id = Uuid::new_v4();

        let err = service(Some(a_post(user_id)))
            .execute(
                UserId::from(user_id),
                Uuid::new_v4(),
                PatchBlogPostData {
                    published_at: BlogPatchField::Value(Utc::now()),
                    content: BlogPatchField::Null,
                    ..Default::default()
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, PatchBlogPostError::InvalidContent(_)));
    }

    /// Editing a draft is untouched by any of this.
    #[tokio::test]
    async fn a_blank_draft_can_still_be_patched() {
        let user_id = Uuid::new_v4();
        let mut blank = a_post(user_id);
        blank.content = "".into();

        assert!(service(Some(blank))
            .execute(
                UserId::from(user_id),
                Uuid::new_v4(),
                PatchBlogPostData {
                    title: BlogPatchField::Value("A better title".into()),
                    ..Default::default()
                },
            )
            .await
            .is_ok());
    }

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

        for bad in [
            "",
            "   ",
            "has space",
            "has/slash",
            "héllo",
            &"a".repeat(201),
        ] {
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
