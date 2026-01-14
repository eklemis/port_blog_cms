use crate::auth::application::services::hash::PasswordHashingService;
use crate::auth::application::services::jwt::JwtService;
use crate::email::application::services::EmailService;
use crate::modules::auth::application::domain::entities::User;
use crate::modules::auth::application::ports::outgoing::{
    user_query::UserQuery, user_repository::UserRepository, UserRepositoryError,
};
use async_trait::async_trait;
use uuid::Uuid;

// Possible errors for creating a user
#[derive(Debug, Clone)]
pub enum CreateUserError {
    UsernameAlreadyExists,
    EmailAlreadyExists,
    HashingFailed(String),
    RepositoryError(String),
}

// Interface for CreateUser use case
#[async_trait]
pub trait ICreateUserUseCase: Send + Sync {
    async fn execute(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> Result<User, CreateUserError>;
}

// Implementation of CreateUser use case
#[derive(Debug, Clone)]
pub struct CreateUserUseCase<Q, R>
where
    Q: UserQuery + Send + Sync,
    R: UserRepository + Send + Sync,
{
    app_url: String,
    query: Q,
    repository: R,
    password_hasher: PasswordHashingService,
    jwt_service: JwtService,
    email_service: EmailService,
}

impl<Q, R> CreateUserUseCase<Q, R>
where
    Q: UserQuery + Send + Sync,
    R: UserRepository + Send + Sync,
{
    pub fn new(
        query: Q,
        repository: R,
        password_hasher: PasswordHashingService,
        jwt_service: JwtService,
        email_service: EmailService,
        app_url: String,
    ) -> Self {
        Self {
            app_url,
            query,
            repository,
            password_hasher,
            jwt_service,
            email_service,
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
        if let Ok(Some(existing_user)) = self.query.find_by_email(&email).await {
            if existing_user.is_deleted {
                // ✅ Reactivate the existing soft-deleted user
                let updated_user = self
                    .repository
                    .restore_user(existing_user.id)
                    .await
                    .map_err(|e| CreateUserError::RepositoryError(e.to_string()))?;
                return Ok(updated_user);
            } else {
                return Err(CreateUserError::EmailAlreadyExists);
            }
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
            is_verified: false,
            is_deleted: false,
        };

        // 5️⃣ **Persist the user in the database**
        match self.repository.create_user(user.clone()).await {
            Ok(user) => {
                // 1️⃣ Generate verification token
                let verification_token = self
                    .jwt_service
                    .generate_verification_token(user.id)
                    .map_err(|e| CreateUserError::RepositoryError(e.to_string()))?;

                // 2️⃣ Send verification email
                #[cfg(not(tarpaulin_include))]
                let verification_link = format!(
                    "{}/api/auth/email-verification/{}",
                    self.app_url, verification_token
                );
                let html_body = format!(
                    r#"
                    <p>Hi {},</p>
                    <p>Welcome to Ekstion! We're excited to have you on board.</p>
                    <p>
                        To complete your registration, click the button below:
                    </p>
                    <p>
                        <a href="{}" style="
                            display: inline-block;
                            padding: 10px 20px;
                            background-color: #007BFF;
                            color: white;
                            text-decoration: none;
                            border-radius: 5px;
                        ">Verify Your Email</a>
                    </p>
                    <p>
                        <strong>Note:</strong> This link is valid for only 1 hour. If it expires, you'll need to register again.
                    </p>
                    <p>Thanks,<br>The Ekstion Team</p>
                    "#,
                    &user.username, verification_link
                );
                self.email_service
                    .send_email(&user.email, "Verify Your Email", &html_body)
                    .await
                    .map_err(|e| CreateUserError::RepositoryError(e.to_string()))?;

                Ok(user)
            }
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
    use crate::modules::auth::application::services::jwt::JwtConfig;
    use crate::modules::email::application::ports::outgoing::EmailSender;
    use async_trait::async_trait;
    use std::sync::Arc;
    use uuid::Uuid;

    // Mock UserQuery
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

    // Mock UserRepository
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
        async fn soft_delete_user(&self, _user_id: Uuid) -> Result<(), UserRepositoryError> {
            unimplemented!()
        }
        async fn restore_user(&self, _user_id: Uuid) -> Result<User, UserRepositoryError> {
            unimplemented!()
        }
        async fn activate_user(&self, _user_id: Uuid) -> Result<(), UserRepositoryError> {
            unimplemented!()
        }
    }

    // Mock Password Hasher
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

    // Mock Email Sender
    struct MockEmailSender;

    impl Default for MockEmailSender {
        fn default() -> Self {
            MockEmailSender
        }
    }

    #[async_trait]
    impl EmailSender for MockEmailSender {
        async fn send_email(&self, _to: &str, _subject: &str, _body: &str) -> Result<(), String> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_create_user_success() {
        // Arrange
        let query = MockUserQuery::default();
        let repository = MockUserRepository::default();
        let password_hasher = PasswordHashingService::with_hasher(MockPasswordHasher);
        let jwt_service = JwtService::new(JwtConfig::from_env());

        let sender = Arc::new(MockEmailSender::default());
        let email_service = EmailService::new(sender);
        let app_url = String::from("0.0.0.0:8080");
        let use_case = CreateUserUseCase::new(
            query,
            repository,
            password_hasher,
            jwt_service,
            email_service,
            app_url,
        );

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
                is_verified: false,
                is_deleted: false,
            }),
            ..Default::default()
        };
        let repository = MockUserRepository::default();
        let password_hasher = PasswordHashingService::with_hasher(MockPasswordHasher);

        // mock jwt service
        let jwt_service = JwtService::new(JwtConfig::from_env());

        // mock email service
        let sender = Arc::new(MockEmailSender::default());
        let email_service = EmailService::new(sender);

        // app url
        let app_url = String::from("0.0.0.0:8080");

        let use_case = CreateUserUseCase::new(
            query,
            repository,
            password_hasher,
            jwt_service,
            email_service,
            app_url,
        );

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
                is_verified: false,
                is_deleted: false,
            }),
            ..Default::default()
        };
        let repository = MockUserRepository::default();
        let password_hasher = PasswordHashingService::with_hasher(MockPasswordHasher);

        // mock jwt service
        let jwt_service = JwtService::new(JwtConfig::from_env());

        // mock email service
        let sender = Arc::new(MockEmailSender::default());
        let email_service = EmailService::new(sender);

        // app url
        let app_url = String::from("0.0.0.0:8080");

        let use_case = CreateUserUseCase::new(
            query,
            repository,
            password_hasher,
            jwt_service,
            email_service,
            app_url,
        );

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
    async fn test_create_user_email_already_exists_active_user() {
        // Arrange - Mock an ACTIVE (not deleted) user with existing email
        let active_user = User {
            id: Uuid::new_v4(),
            username: "active_user".to_string(),
            email: "existing@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            is_verified: false,
            is_deleted: false, // Active user (not deleted)
        };

        let query = MockUserQuery {
            existing_user_by_email: Some(active_user),
            existing_user_by_username: None,
        };

        let repository = MockUserRepository::default();
        let password_hasher = PasswordHashingService::with_hasher(MockPasswordHasher);
        let jwt_service = JwtService::new(JwtConfig::from_env());
        let sender = Arc::new(MockEmailSender::default());
        let email_service = EmailService::new(sender);
        let app_url = String::from("http://localhost:8080");

        let use_case = CreateUserUseCase::new(
            query,
            repository,
            password_hasher,
            jwt_service,
            email_service,
            app_url,
        );

        // Act - Try to create user with existing active email
        let result = use_case
            .execute(
                "new_user".to_string(),
                "existing@example.com".to_string(),
                "password".to_string(),
            )
            .await;

        // Assert - Should return EmailAlreadyExists error
        assert!(
            matches!(result, Err(CreateUserError::EmailAlreadyExists)),
            "Expected EmailAlreadyExists error, got: {:?}",
            result
        );
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
        let jwt_service = JwtService::new(JwtConfig::from_env());

        let sender = Arc::new(MockEmailSender::default());
        let email_service = EmailService::new(sender);
        let app_url = String::new();
        let use_case = CreateUserUseCase::new(
            query,
            repository,
            password_hasher,
            jwt_service,
            email_service,
            app_url,
        );

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
    async fn test_create_user_reactivate_soft_deleted_user() {
        // Arrange - Mock a soft-deleted user with the same email
        let soft_deleted_user = User {
            id: Uuid::new_v4(),
            username: "deleted_user".to_string(),
            email: "deleted@example.com".to_string(),
            password_hash: "old_hash".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            is_verified: false,
            is_deleted: true, // This user is soft-deleted
        };

        let query = MockUserQuery {
            existing_user_by_email: Some(soft_deleted_user.clone()),
            ..Default::default()
        };

        // Mock repository that will restore the user
        #[derive(Default)]
        struct MockUserRepositoryWithRestore;

        #[async_trait]
        impl UserRepository for MockUserRepositoryWithRestore {
            async fn create_user(&self, user: User) -> Result<User, UserRepositoryError> {
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

            async fn soft_delete_user(&self, _user_id: Uuid) -> Result<(), UserRepositoryError> {
                unimplemented!()
            }

            async fn restore_user(&self, user_id: Uuid) -> Result<User, UserRepositoryError> {
                // Return the restored user (is_deleted = false)
                Ok(User {
                    id: user_id,
                    username: "deleted_user".to_string(),
                    email: "deleted@example.com".to_string(),
                    password_hash: "old_hash".to_string(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    is_verified: false,
                    is_deleted: false, // Now restored
                })
            }

            async fn activate_user(&self, _user_id: Uuid) -> Result<(), UserRepositoryError> {
                unimplemented!()
            }
        }

        let repository = MockUserRepositoryWithRestore::default();
        let password_hasher = PasswordHashingService::with_hasher(MockPasswordHasher);
        let jwt_service = JwtService::new(JwtConfig::from_env());
        let sender = Arc::new(MockEmailSender::default());
        let email_service = EmailService::new(sender);
        let app_url = String::from("http://localhost:8080");

        let use_case = CreateUserUseCase::new(
            query,
            repository,
            password_hasher,
            jwt_service,
            email_service,
            app_url,
        );

        // Act - Try to create user with the same email as the soft-deleted user
        let result = use_case
            .execute(
                "new_user".to_string(),
                "deleted@example.com".to_string(),
                "new_password".to_string(),
            )
            .await;

        // Assert - Should successfully restore the user
        assert!(result.is_ok(), "Expected user restoration to succeed");
        let restored_user = result.unwrap();
        assert_eq!(restored_user.email, "deleted@example.com");
        assert_eq!(restored_user.is_deleted, false);
    }
    #[tokio::test]
    async fn test_create_user_restore_user_fails() {
        // Arrange - Mock a soft-deleted user
        let soft_deleted_user = User {
            id: Uuid::new_v4(),
            username: "deleted_user".to_string(),
            email: "deleted@example.com".to_string(),
            password_hash: "old_hash".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            is_verified: false,
            is_deleted: true,
        };

        let query = MockUserQuery {
            existing_user_by_email: Some(soft_deleted_user.clone()),
            ..Default::default()
        };

        // Mock repository that will fail on restore
        #[derive(Default)]
        struct MockUserRepositoryFailRestore;

        #[async_trait]
        impl UserRepository for MockUserRepositoryFailRestore {
            async fn create_user(&self, user: User) -> Result<User, UserRepositoryError> {
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

            async fn soft_delete_user(&self, _user_id: Uuid) -> Result<(), UserRepositoryError> {
                unimplemented!()
            }

            async fn restore_user(&self, _user_id: Uuid) -> Result<User, UserRepositoryError> {
                Err(UserRepositoryError::DatabaseError(
                    "Failed to restore user".to_string(),
                ))
            }

            async fn activate_user(&self, _user_id: Uuid) -> Result<(), UserRepositoryError> {
                unimplemented!()
            }
        }

        let repository = MockUserRepositoryFailRestore;
        let password_hasher = PasswordHashingService::with_hasher(MockPasswordHasher);
        let jwt_service = JwtService::new(JwtConfig::from_env());
        let sender = Arc::new(MockEmailSender::default());
        let email_service = EmailService::new(sender);
        let app_url = String::from("http://localhost:8080");

        let use_case = CreateUserUseCase::new(
            query,
            repository,
            password_hasher,
            jwt_service,
            email_service,
            app_url,
        );

        // Act
        let result = use_case
            .execute(
                "new_user".to_string(),
                "deleted@example.com".to_string(),
                "new_password".to_string(),
            )
            .await;

        // Assert
        assert!(matches!(result, Err(CreateUserError::RepositoryError(_))));
    }
    #[tokio::test]
    async fn test_create_user_repository_error() {
        // Arrange
        let query = MockUserQuery::default();
        let repository = MockUserRepository {
            should_fail_on_create: true,
        };
        let password_hasher = PasswordHashingService::with_hasher(MockPasswordHasher);

        // mock jwt service
        let jwt_service = JwtService::new(JwtConfig::from_env());

        // mock email service
        let sender = Arc::new(MockEmailSender::default());
        let email_service = EmailService::new(sender);

        // app url
        let app_url = String::from("0.0.0.0:8080");

        let use_case = CreateUserUseCase::new(
            query,
            repository,
            password_hasher,
            jwt_service,
            email_service,
            app_url,
        );

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
    #[tokio::test]
    async fn test_create_user_repository_unknown_error() {
        // Arrange
        let query = MockUserQuery::default();

        // Mock repository that returns a non-DatabaseError variant
        #[derive(Default)]
        struct MockUserRepositoryUnknownError;

        #[async_trait]
        impl UserRepository for MockUserRepositoryUnknownError {
            async fn create_user(&self, _user: User) -> Result<User, UserRepositoryError> {
                // Return UserAlreadyExists to trigger the Err(_) catch-all branch
                Err(UserRepositoryError::UserAlreadyExists)
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

            async fn soft_delete_user(&self, _user_id: Uuid) -> Result<(), UserRepositoryError> {
                unimplemented!()
            }

            async fn restore_user(&self, _user_id: Uuid) -> Result<User, UserRepositoryError> {
                unimplemented!()
            }

            async fn activate_user(&self, _user_id: Uuid) -> Result<(), UserRepositoryError> {
                unimplemented!()
            }
        }

        let repository = MockUserRepositoryUnknownError;
        let password_hasher = PasswordHashingService::with_hasher(MockPasswordHasher);
        let jwt_service = JwtService::new(JwtConfig::from_env());
        let sender = Arc::new(MockEmailSender::default());
        let email_service = EmailService::new(sender);
        let app_url = String::from("http://localhost:8080");

        let use_case = CreateUserUseCase::new(
            query,
            repository,
            password_hasher,
            jwt_service,
            email_service,
            app_url,
        );

        // Act
        let result = use_case
            .execute(
                "new_user".to_string(),
                "new_user@example.com".to_string(),
                "password".to_string(),
            )
            .await;

        // Assert - Should return RepositoryError with "Unknown repository error" message
        assert!(matches!(result, Err(CreateUserError::RepositoryError(_))));
        if let Err(CreateUserError::RepositoryError(msg)) = result {
            assert_eq!(msg, "Unknown repository error");
        }
    }
}
