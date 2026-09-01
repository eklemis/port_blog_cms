use async_trait::async_trait;

use crate::blog::application::ports::incoming::use_cases::{
    CreateBlogPostCommand, CreateBlogPostError, CreateBlogPostUseCase,
};
use crate::blog::application::ports::outgoing::{BlogPostRepository, CreateBlogPostData};
use crate::blog::domain::entities::BlogPost;

/// Implements the corresponding use-case contract.
pub struct CreateBlogPostService<R>
where
    R: BlogPostRepository,
{
    repository: R,
}

impl<R> CreateBlogPostService<R>
where
    R: BlogPostRepository,
{
    /// Builds it from the ports it depends on.
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    /// Slug rules are enforced here rather than at the adapter so a bad slug is
    /// a validation error the caller can act on, instead of a database
    /// constraint violation surfacing as a 500.
    fn validate_slug(slug: &str) -> Result<String, CreateBlogPostError> {
        let trimmed = slug.trim().to_lowercase();

        if trimmed.is_empty() {
            return Err(CreateBlogPostError::InvalidSlug(
                "Slug cannot be empty".to_string(),
            ));
        }
        if trimmed.len() > 200 {
            return Err(CreateBlogPostError::InvalidSlug(
                "Slug must not exceed 200 characters".to_string(),
            ));
        }
        if !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err(CreateBlogPostError::InvalidSlug(
                "Slug may contain only letters, numbers, and hyphens".to_string(),
            ));
        }

        Ok(trimmed)
    }

    fn validate_title(title: &str) -> Result<String, CreateBlogPostError> {
        let trimmed = title.trim();

        if trimmed.is_empty() {
            return Err(CreateBlogPostError::InvalidTitle(
                "Title cannot be empty".to_string(),
            ));
        }
        if trimmed.chars().count() > 200 {
            return Err(CreateBlogPostError::InvalidTitle(
                "Title must not exceed 200 characters".to_string(),
            ));
        }

        Ok(trimmed.to_string())
    }

    fn validate_content(content: &str) -> Result<(), CreateBlogPostError> {
        if content.trim().is_empty() {
            return Err(CreateBlogPostError::InvalidContent(
                "Content cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl<R> CreateBlogPostUseCase for CreateBlogPostService<R>
where
    R: BlogPostRepository + Send + Sync,
{
    async fn execute(
        &self,
        command: CreateBlogPostCommand,
    ) -> Result<BlogPost, CreateBlogPostError> {
        let title = Self::validate_title(&command.title)?;
        let slug = Self::validate_slug(&command.slug)?;
        Self::validate_content(&command.content)?;

        self.repository
            .create(CreateBlogPostData {
                owner: command.owner,
                title,
                slug,
                excerpt: command
                    .excerpt
                    .map(|e| e.trim().to_string())
                    .filter(|e| !e.is_empty()),
                content: command.content,
                published_at: command.published_at,
            })
            .await
            .map_err(CreateBlogPostError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::domain::entities::UserId;
    use crate::blog::application::ports::outgoing::{BlogPostRepositoryError, PatchBlogPostData};
    use chrono::Utc;
    use std::sync::Mutex;
    use uuid::Uuid;

    struct MockRepo {
        result: Result<BlogPost, BlogPostRepositoryError>,
        seen: Mutex<Option<CreateBlogPostData>>,
    }

    impl MockRepo {
        fn ok() -> Self {
            let now = Utc::now();
            Self {
                result: Ok(BlogPost {
                    id: Uuid::new_v4(),
                    user_id: Uuid::new_v4(),
                    title: "Hello".into(),
                    slug: "hello".into(),
                    excerpt: None,
                    content: "body".into(),
                    published_at: None,
                    created_at: now,
                    updated_at: now,
                }),
                seen: Mutex::new(None),
            }
        }

        fn err(e: BlogPostRepositoryError) -> Self {
            Self {
                result: Err(e),
                seen: Mutex::new(None),
            }
        }
    }

    #[async_trait]
    impl BlogPostRepository for MockRepo {
        async fn create(
            &self,
            data: CreateBlogPostData,
        ) -> Result<BlogPost, BlogPostRepositoryError> {
            *self.seen.lock().unwrap() = Some(data);
            self.result.clone()
        }
        async fn fetch_by_id(
            &self,
            _id: Uuid,
        ) -> Result<Option<BlogPost>, BlogPostRepositoryError> {
            unimplemented!()
        }
        async fn patch(
            &self,
            _id: Uuid,
            _d: PatchBlogPostData,
        ) -> Result<BlogPost, BlogPostRepositoryError> {
            unimplemented!()
        }
    }

    fn command(title: &str, slug: &str, content: &str) -> CreateBlogPostCommand {
        CreateBlogPostCommand {
            owner: UserId::from(Uuid::new_v4()),
            title: title.to_string(),
            slug: slug.to_string(),
            excerpt: Some("   ".to_string()),
            content: content.to_string(),
            published_at: None,
        }
    }

    #[tokio::test]
    async fn creates_a_post_with_a_valid_command() {
        let svc = CreateBlogPostService::new(MockRepo::ok());
        assert!(svc.execute(command("Hello", "hello", "body")).await.is_ok());
    }

    #[tokio::test]
    async fn normalises_the_slug_before_writing() {
        let svc = CreateBlogPostService::new(MockRepo::ok());
        svc.execute(command("Hello", "  My-First-POST  ", "body"))
            .await
            .unwrap();

        let seen = svc.repository.seen.lock().unwrap().clone().unwrap();
        assert_eq!(seen.slug, "my-first-post");
    }

    /// A whitespace-only excerpt is dropped rather than stored, so clients do
    /// not have to distinguish "" from absent when rendering.
    #[tokio::test]
    async fn blank_excerpts_become_none() {
        let svc = CreateBlogPostService::new(MockRepo::ok());
        svc.execute(command("Hello", "hello", "body"))
            .await
            .unwrap();

        let seen = svc.repository.seen.lock().unwrap().clone().unwrap();
        assert_eq!(seen.excerpt, None);
    }

    #[tokio::test]
    async fn rejects_an_empty_title() {
        let svc = CreateBlogPostService::new(MockRepo::ok());
        let err = svc
            .execute(command("   ", "hello", "body"))
            .await
            .unwrap_err();
        assert!(matches!(err, CreateBlogPostError::InvalidTitle(_)));
    }

    #[tokio::test]
    async fn rejects_an_empty_slug() {
        let svc = CreateBlogPostService::new(MockRepo::ok());
        let err = svc
            .execute(command("Hello", "  ", "body"))
            .await
            .unwrap_err();
        assert!(matches!(err, CreateBlogPostError::InvalidSlug(_)));
    }

    /// Slugs go in URLs, so anything outside `[a-z0-9-]` is rejected up front
    /// rather than producing a link that needs escaping.
    #[tokio::test]
    async fn rejects_a_slug_with_unsafe_characters() {
        let svc = CreateBlogPostService::new(MockRepo::ok());
        for bad in ["hello world", "hello/world", "héllo", "hello?x=1"] {
            let err = svc
                .execute(command("Hello", bad, "body"))
                .await
                .unwrap_err();
            assert!(
                matches!(err, CreateBlogPostError::InvalidSlug(_)),
                "{bad} should be rejected"
            );
        }
    }

    #[tokio::test]
    async fn rejects_empty_content() {
        let svc = CreateBlogPostService::new(MockRepo::ok());
        let err = svc
            .execute(command("Hello", "hello", "  "))
            .await
            .unwrap_err();
        assert!(matches!(err, CreateBlogPostError::InvalidContent(_)));
    }

    #[tokio::test]
    async fn maps_a_slug_collision_to_its_own_error() {
        let svc =
            CreateBlogPostService::new(MockRepo::err(BlogPostRepositoryError::SlugAlreadyExists));
        let err = svc
            .execute(command("Hello", "hello", "body"))
            .await
            .unwrap_err();
        assert!(matches!(err, CreateBlogPostError::SlugAlreadyExists));
    }
}
