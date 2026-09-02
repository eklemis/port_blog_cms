//! Assembles an author's public profile.

use async_trait::async_trait;
use std::sync::Arc;

use crate::auth::application::ports::outgoing::user_query::UserQuery;
use crate::auth::application::use_cases::get_public_profile::{
    GetPublicProfileError, GetPublicProfileUseCase, PublicProfile,
};
use crate::multimedia::application::domain::entities::PublicMedia;

/// Loads a user's avatar, if they have one.
///
/// A port rather than a direct call so `auth` does not reach into the media
/// schema — it says what it needs and the composition root supplies it.
#[async_trait]
pub trait AvatarLoader: Send + Sync {
    /// The user's avatar, or `None`.
    async fn load(&self, user_id: uuid::Uuid) -> Result<Option<PublicMedia>, String>;
}

/// Implements the corresponding use-case contract.
pub struct GetPublicProfileService<Q> {
    user_query: Q,
    avatars: Arc<dyn AvatarLoader>,
}

impl<Q> GetPublicProfileService<Q> {
    /// Builds it from the ports it depends on.
    pub fn new(user_query: Q, avatars: Arc<dyn AvatarLoader>) -> Self {
        Self {
            user_query,
            avatars,
        }
    }
}

#[async_trait]
impl<Q: UserQuery + Send + Sync> GetPublicProfileUseCase for GetPublicProfileService<Q> {
    async fn execute(&self, username: &str) -> Result<PublicProfile, GetPublicProfileError> {
        let user = self
            .user_query
            .find_by_username(username.trim())
            .await
            .map_err(|e| GetPublicProfileError::QueryError(e.to_string()))?
            .ok_or(GetPublicProfileError::NotFound)?;

        // UserQuery does not filter deleted accounts — see its documentation —
        // so this check is the caller's job and skipping it would serve a
        // deleted author's profile to readers.
        if user.is_deleted {
            return Err(GetPublicProfileError::NotFound);
        }

        // A failure to load the avatar degrades to "no avatar" rather than
        // failing the page: a reader would rather see the author's name without
        // a portrait than a 500.
        let avatar = match self.avatars.load(user.id).await {
            Ok(avatar) => avatar,
            Err(e) => {
                tracing::warn!("Failed to load avatar for a public profile: {}", e);
                None
            }
        };

        Ok(PublicProfile {
            username: user.username,
            full_name: user.full_name,
            bio: user.bio,
            avatar,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::ports::outgoing::user_query::{UserQueryError, UserQueryResult};
    use chrono::Utc;
    use uuid::Uuid;

    fn a_user(deleted: bool, bio: Option<&str>) -> UserQueryResult {
        UserQueryResult {
            id: Uuid::new_v4(),
            email: "jane@example.com".into(),
            username: "janedoe".into(),
            password_hash: "hash".into(),
            full_name: "Jane Doe".into(),
            bio: bio.map(str::to_string),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_verified: true,
            is_deleted: deleted,
        }
    }

    struct StubQuery(Option<UserQueryResult>);

    #[async_trait]
    impl UserQuery for StubQuery {
        async fn find_by_username(
            &self,
            _u: &str,
        ) -> Result<Option<UserQueryResult>, UserQueryError> {
            Ok(self.0.clone())
        }
        async fn find_by_id(&self, _i: Uuid) -> Result<Option<UserQueryResult>, UserQueryError> {
            unimplemented!()
        }
        async fn find_by_email(&self, _e: &str) -> Result<Option<UserQueryResult>, UserQueryError> {
            unimplemented!()
        }
    }

    struct StubAvatars(Result<Option<PublicMedia>, String>);

    #[async_trait]
    impl AvatarLoader for StubAvatars {
        async fn load(&self, _u: Uuid) -> Result<Option<PublicMedia>, String> {
            self.0.clone()
        }
    }

    fn svc(
        user: Option<UserQueryResult>,
        avatar: Result<Option<PublicMedia>, String>,
    ) -> GetPublicProfileService<StubQuery> {
        GetPublicProfileService::new(StubQuery(user), Arc::new(StubAvatars(avatar)))
    }

    #[tokio::test]
    async fn it_returns_the_author_without_their_email() {
        let profile = svc(Some(a_user(false, Some("Rust, mostly."))), Ok(None))
            .execute("janedoe")
            .await
            .unwrap();

        assert_eq!(profile.username, "janedoe");
        assert_eq!(profile.full_name, "Jane Doe");
        assert_eq!(profile.bio.as_deref(), Some("Rust, mostly."));

        // The type carries no email field at all, which is the point: this is
        // the one endpoint serving one user's details to another.
        let json = serde_json::to_string(&profile).unwrap();
        assert!(
            !json.contains("jane@example.com"),
            "a public profile must not leak the account's email: {json}"
        );
    }

    /// `UserQuery` does not filter deleted accounts — see its documentation —
    /// so forgetting this check would serve a deleted author's profile to
    /// readers while the rest of their public surface 404s.
    #[tokio::test]
    async fn a_deleted_account_is_not_found() {
        let err = svc(Some(a_user(true, None)), Ok(None))
            .execute("janedoe")
            .await
            .unwrap_err();

        assert!(matches!(err, GetPublicProfileError::NotFound));
    }

    #[tokio::test]
    async fn an_unknown_username_is_not_found() {
        let err = svc(None, Ok(None)).execute("nobody").await.unwrap_err();

        assert!(matches!(err, GetPublicProfileError::NotFound));
    }

    /// A page that cannot load a portrait should still introduce the author.
    /// Failing the whole request would take a public page down over an image.
    #[tokio::test]
    async fn an_avatar_failure_degrades_to_no_avatar() {
        let profile = svc(Some(a_user(false, None)), Err("storage down".into()))
            .execute("janedoe")
            .await
            .expect("the profile must still be served");

        assert!(profile.avatar.is_none());
        assert_eq!(profile.full_name, "Jane Doe");
    }
}
