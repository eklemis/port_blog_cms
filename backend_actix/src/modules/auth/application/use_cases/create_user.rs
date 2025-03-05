use crate::modules::auth::application::domain::entities::User;
use crate::modules::auth::application::ports::outgoing::{
    user_query::UserQuery, user_repository::UserRepository, UserRepositoryError,
};
use crate::modules::auth::application::services::hash::PasswordHashingService;
use async_trait::async_trait;
use uuid::Uuid;

/// Possible errors for creating a user
#[derive(Debug)]
pub enum CreateUserError {
    UsernameAlreadyExists,
    EmailAlreadyExists,
    HashingFailed(String),
    RepositoryError(String),
}

/// Interface for CreateUser use case
#[async_trait]
pub trait ICreateUserUseCase {
    async fn execute(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> Result<User, CreateUserError>;
}

/// Implementation of CreateUser use case
#[derive(Debug, Clone)]
pub struct CreateUserUseCase<Q, R>
where
    Q: UserQuery + Send + Sync,
    R: UserRepository + Send + Sync,
{
    query: Q,
    repository: R,
    password_hasher: PasswordHashingService,
}

impl<Q, R> CreateUserUseCase<Q, R>
where
    Q: UserQuery + Send + Sync,
    R: UserRepository + Send + Sync,
{
    pub fn new(query: Q, repository: R, password_hasher: PasswordHashingService) -> Self {
        Self {
            query,
            repository,
            password_hasher,
        }
    }
}

#[async_trait]
impl<Q, R> ICreateUserUseCase for CreateUserUseCase<Q, R>
where
    Q: UserQuery + Send + Sync,
    R: UserRepository + Send + Sync,
{
    async fn execute(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> Result<User, CreateUserError> {
        // 1️⃣ **Check if username already exists**
        if let Ok(Some(_)) = self.query.find_by_username(&username).await {
            return Err(CreateUserError::UsernameAlreadyExists);
        }

        // 2️⃣ **Check if email already exists**
        if let Ok(Some(_)) = self.query.find_by_email(&email).await {
            return Err(CreateUserError::EmailAlreadyExists);
        }

        // 3️⃣ **Hash password**
        let password_hash = self
            .password_hasher
            .hash_password(password)
            .await
            .map_err(|e| CreateUserError::HashingFailed(e))?;

        // 4️⃣ **Create User Entity**
        let user = User {
            id: Uuid::new_v4(),
            username,
            email,
            password_hash,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // 5️⃣ **Persist the user in the database**
        match self.repository.create_user(user.clone()).await {
            Ok(user) => Ok(user),
            Err(UserRepositoryError::DatabaseError(e)) => Err(CreateUserError::RepositoryError(e)),
            Err(_) => Err(CreateUserError::RepositoryError(
                "Unknown repository error".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::auth::application::ports::outgoing::{
        user_query::UserQuery, user_repository::UserRepository, UserRepositoryError,
    };
    use crate::modules::auth::application::services::hash::password_hasher::PasswordHasher;
    use crate::modules::auth::application::services::hash::PasswordHashingService;
    use async_trait::async_trait;
    use uuid::Uuid;

    /// Mock UserQuery
    #[derive(Default)]
    struct MockUserQuery {
        existing_user_by_username: Option<User>,
        existing_user_by_email: Option<User>,
    }

    #[async_trait]
    impl UserQuery for MockUserQuery {
        async fn find_by_id(&self, _user_id: Uuid) -> Result<Option<User>, String> {
            Ok(None)
        }

        async fn find_by_username(&self, username: &str) -> Result<Option<User>, String> {
            if let Some(user) = &self.existing_user_by_username {
                if user.username == username {
                    return Ok(Some(user.clone()));
                }
            }
            Ok(None)
        }

        async fn find_by_email(&self, email: &str) -> Result<Option<User>, String> {
            if let Some(user) = &self.existing_user_by_email {
                if user.email == email {
                    return Ok(Some(user.clone()));
                }
            }
            Ok(None)
        }
    }

    /// Mock UserRepository
    #[derive(Default)]
    struct MockUserRepository {
        should_fail_on_create: bool,
    }

    #[async_trait]
    impl UserRepository for MockUserRepository {
        async fn create_user(&self, user: User) -> Result<User, UserRepositoryError> {
            if self.should_fail_on_create {
                return Err(UserRepositoryError::DatabaseError(
                    "DB insert failed".to_string(),
                ));
            }
            Ok(user)
        }

        async fn update_password(
            &self,
            _user_id: Uuid,
            _new_password_hash: String,
        ) -> Result<(), UserRepositoryError> {
            unimplemented!()
        }

        async fn delete_user(&self, _user_id: Uuid) -> Result<(), UserRepositoryError> {
            unimplemented!()
        }
    }

    /// Mock Password Hasher
    #[derive(Debug)]
    struct MockPasswordHasher;

    #[async_trait]
    impl PasswordHasher for MockPasswordHasher {
        fn hash_password(&self, _password: &str) -> Result<String, String> {
            Ok("hashed_password".to_string())
        }

        fn verify_password(&self, _password: &str, _hash: &str) -> Result<bool, String> {
            Ok(true)
        }
    }

    #[tokio::test]
    async fn test_create_user_success() {
        // Arrange
        let query = MockUserQuery::default();
        let repository = MockUserRepository::default();
        let password_hasher = PasswordHashingService::with_hasher(MockPasswordHasher);
        let use_case = CreateUserUseCase::new(query, repository, password_hasher);

        // Act
        let result = use_case
            .execute(
                "new_user".to_string(),
                "new_user@example.com".to_string(),
                "password".to_string(),
            )
            .await;

        // Assert
        assert!(result.is_ok(), "Expected user creation to succeed");
        let created_user = result.unwrap();
        assert_eq!(created_user.username, "new_user");
        assert_eq!(created_user.email, "new_user@example.com");
        assert_eq!(created_user.password_hash, "hashed_password");
    }

    #[tokio::test]
    async fn test_create_user_username_already_exists() {
        // Arrange
        let query = MockUserQuery {
            existing_user_by_username: Some(User {
                id: Uuid::new_v4(),
                username: "existing_user".to_string(),
                email: "existing@example.com".to_string(),
                password_hash: "hashed_password".to_string(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }),
            ..Default::default()
        };
        let repository = MockUserRepository::default();
        let password_hasher = PasswordHashingService::with_hasher(MockPasswordHasher);
        let use_case = CreateUserUseCase::new(query, repository, password_hasher);

        // Act
        let result = use_case
            .execute(
                "existing_user".to_string(),
                "new_user@example.com".to_string(),
                "password".to_string(),
            )
            .await;

        // Assert
        assert!(matches!(
            result,
            Err(CreateUserError::UsernameAlreadyExists)
        ));
    }

    #[tokio::test]
    async fn test_create_user_email_already_exists() {
        // Arrange
        let query = MockUserQuery {
            existing_user_by_email: Some(User {
                id: Uuid::new_v4(),
                username: "another_user".to_string(),
                email: "existing@example.com".to_string(),
                password_hash: "hashed_password".to_string(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }),
            ..Default::default()
        };
        let repository = MockUserRepository::default();
        let password_hasher = PasswordHashingService::with_hasher(MockPasswordHasher);
        let use_case = CreateUserUseCase::new(query, repository, password_hasher);

        // Act
        let result = use_case
            .execute(
                "new_user".to_string(),
                "existing@example.com".to_string(),
                "password".to_string(),
            )
            .await;

        // Assert
        assert!(matches!(result, Err(CreateUserError::EmailAlreadyExists)));
    }

    #[tokio::test]
    async fn test_create_user_password_hashing_fails() {
        // Arrange
        #[derive(Debug)]
        struct FailingPasswordHasher;

        #[async_trait]
        impl PasswordHasher for FailingPasswordHasher {
            fn hash_password(&self, _password: &str) -> Result<String, String> {
                Err("Hashing failed".to_string())
            }

            fn verify_password(&self, _password: &str, _hash: &str) -> Result<bool, String> {
                Ok(false)
            }
        }

        let query = MockUserQuery::default();
        let repository = MockUserRepository::default();
        let password_hasher = PasswordHashingService::with_hasher(FailingPasswordHasher);
        let use_case = CreateUserUseCase::new(query, repository, password_hasher);

        // Act
        let result = use_case
            .execute(
                "new_user".to_string(),
                "new_user@example.com".to_string(),
                "password".to_string(),
            )
            .await;

        // Assert
        assert!(matches!(result, Err(CreateUserError::HashingFailed(_))));
    }

    #[tokio::test]
    async fn test_create_user_repository_error() {
        // Arrange
        let query = MockUserQuery::default();
        let repository = MockUserRepository {
            should_fail_on_create: true,
        };
        let password_hasher = PasswordHashingService::with_hasher(MockPasswordHasher);
        let use_case = CreateUserUseCase::new(query, repository, password_hasher);

        // Act
        let result = use_case
            .execute(
                "new_user".to_string(),
                "new_user@example.com".to_string(),
                "password".to_string(),
            )
            .await;

        // Assert
        assert!(matches!(result, Err(CreateUserError::RepositoryError(_))));
    }
}
