use async_trait::async_trait;
use serde::{Deserialize, Deserializer, Serialize};

use crate::auth::application::{
    ports::outgoing::UserQuery,
    services::{hash::PasswordHashingService, jwt::JwtService},
};
use email_address::EmailAddress;
// ========================= Login Request =========================
/// Validated login request - can be deserialized directly from JSON
#[derive(Debug, Clone)]
pub struct LoginRequest {
    email: String,    // Private - guaranteed valid
    password: String, // Private - guaranteed valid
}

#[derive(Debug, Clone)]
pub enum LoginRequestError {
    EmptyEmail,
    InvalidEmailFormat,
    EmptyPassword,
}

impl std::fmt::Display for LoginRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoginRequestError::EmptyEmail => write!(f, "Email cannot be empty"),
            LoginRequestError::InvalidEmailFormat => write!(f, "Invalid email format"),
            LoginRequestError::EmptyPassword => write!(f, "Password cannot be empty"),
        }
    }
}

impl std::error::Error for LoginRequestError {}

impl LoginRequest {
    /// Create a validated LoginRequest
    pub fn new(email: String, password: String) -> Result<Self, LoginRequestError> {
        let email = Self::validate_email(email)?;
        let password = Self::validate_password(password)?;

        Ok(Self { email, password })
    }

    /// Get email (guaranteed to be valid)
    pub fn email(&self) -> &str {
        &self.email
    }

    /// Get password (guaranteed to be strong & non-empty)
    pub fn password(&self) -> &str {
        &self.password
    }

    // ------------------------
    // Validation helpers
    // ------------------------

    fn validate_email(email: String) -> Result<String, LoginRequestError> {
        let email = email.trim();

        if email.is_empty() {
            return Err(LoginRequestError::EmptyEmail);
        }

        if !EmailAddress::is_valid(email) {
            return Err(LoginRequestError::InvalidEmailFormat);
        }

        Ok(email.to_lowercase())
    }

    fn validate_password(password: String) -> Result<String, LoginRequestError> {
        let password = password.trim();

        if password.is_empty() {
            return Err(LoginRequestError::EmptyPassword);
        }

        Ok(password.to_string())
    }
}

// Custom deserialization that validates during parsing
impl<'de> Deserialize<'de> for LoginRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct LoginRequestHelper {
            email: String,
            password: String,
        }

        let helper = LoginRequestHelper::deserialize(deserializer)?;
        LoginRequest::new(helper.email, helper.password).map_err(serde::de::Error::custom)
    }
}

// ====================== Login Error =============================
#[derive(Debug, Clone)]
pub enum LoginError {
    InvalidCredentials,
    UserDeleted,
    PasswordVerificationFailed(String),
    TokenGenerationFailed(String),
    QueryError(String),
}

impl std::fmt::Display for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoginError::InvalidCredentials => write!(f, "Invalid email or password"),
            LoginError::UserDeleted => write!(f, "User account has been deleted"),
            LoginError::PasswordVerificationFailed(msg) => {
                write!(f, "Password verification failed: {}", msg)
            }
            LoginError::TokenGenerationFailed(msg) => {
                write!(f, "Token generation failed: {}", msg)
            }
            LoginError::QueryError(msg) => write!(f, "Query error: {}", msg),
        }
    }
}

impl std::error::Error for LoginError {}

// ============================ Login Response =================================
#[derive(Debug, Clone, Serialize)]
pub struct UserInfo {
    pub id: uuid::Uuid,
    pub username: String,
    pub email: String,
    pub is_verified: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoginUserResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user: UserInfo,
}

// ============================ Login User Use Case =============================
// Interface for Login use case
#[async_trait]
pub trait ILoginUserUseCase: Send + Sync {
    async fn execute(&self, request: LoginRequest) -> Result<LoginUserResponse, LoginError>;
}

// Implementation of Login use case
#[derive(Debug, Clone)]
pub struct LoginUserUseCase<Q>
where
    Q: UserQuery + Send + Sync,
{
    query: Q,
    password_hasher: PasswordHashingService,
    jwt_service: JwtService,
}

impl<Q> LoginUserUseCase<Q>
where
    Q: UserQuery + Send + Sync,
{
    pub fn new(query: Q, password_hasher: PasswordHashingService, jwt_service: JwtService) -> Self {
        Self {
            query,
            password_hasher,
            jwt_service,
        }
    }
}

#[async_trait]
impl<Q> ILoginUserUseCase for LoginUserUseCase<Q>
where
    Q: UserQuery + Send + Sync,
{
    async fn execute(&self, request: LoginRequest) -> Result<LoginUserResponse, LoginError> {
        // 1️⃣ **Find user by email** (email is already normalized)
        let user = self
            .query
            .find_by_email(request.email())
            .await
            .map_err(|e| LoginError::QueryError(e))?
            .ok_or(LoginError::InvalidCredentials)?;

        // 2️⃣ **Check if user is deleted**
        if user.is_deleted {
            return Err(LoginError::UserDeleted);
        }

        // 3️⃣ **Verify password**
        let is_valid = self
            .password_hasher
            .verify_password(request.password().to_string(), user.password_hash.clone())
            .await
            .map_err(|e| LoginError::PasswordVerificationFailed(e))?;

        if !is_valid {
            return Err(LoginError::InvalidCredentials);
        }

        // 4️⃣ **Generate tokens**
        let access_token = self
            .jwt_service
            .generate_access_token(user.id, user.is_verified)
            .map_err(|e| LoginError::TokenGenerationFailed(e.to_string()))?;

        let refresh_token = self
            .jwt_service
            .generate_refresh_token(user.id, user.is_verified)
            .map_err(|e| LoginError::TokenGenerationFailed(e.to_string()))?;

        // 6️⃣ **Return response**
        Ok(LoginUserResponse {
            access_token,
            refresh_token,
            user: UserInfo {
                id: user.id,
                username: user.username,
                email: user.email,
                is_verified: user.is_verified,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::auth::application::domain::entities::User;
    use crate::modules::auth::application::services::hash::password_hasher::PasswordHasher;
    use crate::modules::auth::application::services::jwt::JwtConfig;
    use async_trait::async_trait;
    use serde_json::json;
    use uuid::Uuid;

    // ==================== LoginRequest Tests ====================
    #[test]
    fn test_login_request_valid() {
        let request = LoginRequest::new("test@example.com".to_string(), "password123".to_string());

        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.email(), "test@example.com");
        assert_eq!(req.password(), "password123");
    }

    #[test]
    fn test_login_request_email_normalized() {
        let request = LoginRequest::new(
            "  Test@Example.COM  ".to_string(),
            "password123".to_string(),
        )
        .unwrap();

        assert_eq!(request.email(), "test@example.com");
    }

    #[test]
    fn test_login_request_empty_email() {
        let result = LoginRequest::new("".to_string(), "password123".to_string());
        assert!(matches!(result, Err(LoginRequestError::EmptyEmail)));
    }

    #[test]
    fn test_login_request_invalid_email_format() {
        let result = LoginRequest::new("invalid-email".to_string(), "password123".to_string());
        assert!(matches!(result, Err(LoginRequestError::InvalidEmailFormat)));
    }

    #[test]
    fn test_login_request_empty_password() {
        let result = LoginRequest::new("test@example.com".to_string(), "".to_string());
        assert!(matches!(result, Err(LoginRequestError::EmptyPassword)));
    }

    #[test]
    fn test_login_request_deserialize_valid() {
        let json = json!({
            "email": "test@example.com",
            "password": "password123"
        });

        let request: LoginRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.email(), "test@example.com");
        assert_eq!(request.password(), "password123");
    }

    #[test]
    fn test_login_request_deserialize_invalid_email() {
        let json = json!({
            "email": "invalid-email",
            "password": "password123"
        });

        let result: Result<LoginRequest, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_login_request_error_display() {
        assert_eq!(
            LoginRequestError::EmptyEmail.to_string(),
            "Email cannot be empty"
        );
        assert_eq!(
            LoginRequestError::InvalidEmailFormat.to_string(),
            "Invalid email format"
        );
        assert_eq!(
            LoginRequestError::EmptyPassword.to_string(),
            "Password cannot be empty"
        );
    }

    // ==================== LoginError Tests ====================
    #[test]
    fn test_login_error_display() {
        assert_eq!(
            LoginError::InvalidCredentials.to_string(),
            "Invalid email or password"
        );
        assert_eq!(
            LoginError::UserDeleted.to_string(),
            "User account has been deleted"
        );
    }

    // ==================== LoginUserUseCase Tests ====================

    // Mock UserQuery
    #[derive(Default)]
    struct MockUserQuery {
        user: Option<User>,
        should_fail: bool,
    }

    #[async_trait]
    impl UserQuery for MockUserQuery {
        async fn find_by_id(&self, _user_id: Uuid) -> Result<Option<User>, String> {
            Ok(None)
        }

        async fn find_by_username(&self, _username: &str) -> Result<Option<User>, String> {
            Ok(None)
        }

        async fn find_by_email(&self, email: &str) -> Result<Option<User>, String> {
            if self.should_fail {
                return Err("Database error".to_string());
            }

            if let Some(user) = &self.user {
                if user.email == email {
                    return Ok(Some(user.clone()));
                }
            }
            Ok(None)
        }
    }

    // Mock Password Hasher
    #[derive(Debug)]
    struct MockPasswordHasher {
        should_verify: bool,
    }

    impl PasswordHasher for MockPasswordHasher {
        fn hash_password(&self, _password: &str) -> Result<String, String> {
            Ok("hashed_password".to_string())
        }

        fn verify_password(&self, _password: &str, _hash: &str) -> Result<bool, String> {
            Ok(self.should_verify)
        }
    }

    // Helper to create JWT service
    fn create_jwt_service() -> JwtService {
        JwtService::new(JwtConfig {
            secret_key: "test_secret_key_min_32_characters_long".to_string(),
            issuer: "testapp".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
        })
    }

    // Helper to create a test user
    fn create_test_user(is_verified: bool, is_deleted: bool) -> User {
        User {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            is_verified,
            is_deleted,
        }
    }

    #[tokio::test]
    async fn test_login_success() {
        let user = create_test_user(true, false);
        let query = MockUserQuery {
            user: Some(user.clone()),
            should_fail: false,
        };
        let password_hasher = PasswordHashingService::with_hasher(MockPasswordHasher {
            should_verify: true,
        });
        let jwt_service = create_jwt_service();

        let use_case = LoginUserUseCase::new(query, password_hasher, jwt_service);

        let request =
            LoginRequest::new("test@example.com".to_string(), "password123".to_string()).unwrap();

        let result = use_case.execute(request).await;

        assert!(result.is_ok(), "Expected successful login");
        let response = result.unwrap();
        assert!(!response.access_token.is_empty());
        assert!(!response.refresh_token.is_empty());
        assert_eq!(response.user.email, "test@example.com");
        assert_eq!(response.user.username, "testuser");
        assert_eq!(response.user.is_verified, true);
    }

    #[tokio::test]
    async fn test_login_user_not_found() {
        let query = MockUserQuery::default();
        let password_hasher = PasswordHashingService::with_hasher(MockPasswordHasher {
            should_verify: true,
        });
        let jwt_service = create_jwt_service();

        let use_case = LoginUserUseCase::new(query, password_hasher, jwt_service);

        let request = LoginRequest::new(
            "nonexistent@example.com".to_string(),
            "password123".to_string(),
        )
        .unwrap();

        let result = use_case.execute(request).await;

        assert!(
            matches!(result, Err(LoginError::InvalidCredentials)),
            "Expected InvalidCredentials, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_login_invalid_password() {
        let user = create_test_user(true, false);
        let query = MockUserQuery {
            user: Some(user),
            should_fail: false,
        };
        let password_hasher = PasswordHashingService::with_hasher(MockPasswordHasher {
            should_verify: false,
        });
        let jwt_service = create_jwt_service();

        let use_case = LoginUserUseCase::new(query, password_hasher, jwt_service);

        let request =
            LoginRequest::new("test@example.com".to_string(), "wrongpassword".to_string()).unwrap();

        let result = use_case.execute(request).await;

        assert!(
            matches!(result, Err(LoginError::InvalidCredentials)),
            "Expected InvalidCredentials, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_login_user_deleted() {
        let user = create_test_user(true, true);
        let query = MockUserQuery {
            user: Some(user),
            should_fail: false,
        };
        let password_hasher = PasswordHashingService::with_hasher(MockPasswordHasher {
            should_verify: true,
        });
        let jwt_service = create_jwt_service();

        let use_case = LoginUserUseCase::new(query, password_hasher, jwt_service);

        let request =
            LoginRequest::new("test@example.com".to_string(), "password123".to_string()).unwrap();

        let result = use_case.execute(request).await;

        assert!(
            matches!(result, Err(LoginError::UserDeleted)),
            "Expected UserDeleted, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_login_query_error() {
        let query = MockUserQuery {
            user: None,
            should_fail: true,
        };
        let password_hasher = PasswordHashingService::with_hasher(MockPasswordHasher {
            should_verify: true,
        });
        let jwt_service = create_jwt_service();

        let use_case = LoginUserUseCase::new(query, password_hasher, jwt_service);

        let request =
            LoginRequest::new("test@example.com".to_string(), "password123".to_string()).unwrap();

        let result = use_case.execute(request).await;

        assert!(
            matches!(result, Err(LoginError::QueryError(_))),
            "Expected QueryError, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_login_password_verification_error() {
        #[derive(Debug)]
        struct FailingPasswordHasher;

        impl PasswordHasher for FailingPasswordHasher {
            fn hash_password(&self, _password: &str) -> Result<String, String> {
                Ok("hash".to_string())
            }

            fn verify_password(&self, _password: &str, _hash: &str) -> Result<bool, String> {
                Err("Verification error".to_string())
            }
        }

        let user = create_test_user(true, false);
        let query = MockUserQuery {
            user: Some(user),
            should_fail: false,
        };
        let password_hasher = PasswordHashingService::with_hasher(FailingPasswordHasher);
        let jwt_service = create_jwt_service();

        let use_case = LoginUserUseCase::new(query, password_hasher, jwt_service);

        let request =
            LoginRequest::new("test@example.com".to_string(), "password123".to_string()).unwrap();

        let result = use_case.execute(request).await;

        assert!(
            matches!(result, Err(LoginError::PasswordVerificationFailed(_))),
            "Expected PasswordVerificationFailed, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_login_email_case_insensitive() {
        let user = create_test_user(true, false);
        let query = MockUserQuery {
            user: Some(user),
            should_fail: false,
        };
        let password_hasher = PasswordHashingService::with_hasher(MockPasswordHasher {
            should_verify: true,
        });
        let jwt_service = create_jwt_service();

        let use_case = LoginUserUseCase::new(query, password_hasher, jwt_service);

        // Login with uppercase email
        let request =
            LoginRequest::new("Test@Example.COM".to_string(), "password123".to_string()).unwrap();

        let result = use_case.execute(request).await;
        assert!(result.is_ok(), "Should succeed with normalized email");
    }
}
