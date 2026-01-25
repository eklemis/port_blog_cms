use async_trait::async_trait;
use bcrypt::{hash, verify, DEFAULT_COST};

use crate::auth::application::ports::outgoing::password_hasher::{HashError, PasswordHasher};

#[derive(Clone)]
pub struct BcryptHasher;

impl BcryptHasher {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PasswordHasher for BcryptHasher {
    async fn hash_password(&self, password: &str) -> Result<String, HashError> {
        let password = password.to_string();

        tokio::task::spawn_blocking(move || {
            hash(password, DEFAULT_COST).map_err(|_| HashError::HashFailed)
        })
        .await
        .map_err(|_| HashError::TaskFailed)?
    }

    async fn verify_password(&self, password: &str, hashed: &str) -> Result<bool, HashError> {
        let password = password.to_string();
        let hashed = hashed.to_string();

        tokio::task::spawn_blocking(move || {
            verify(password, &hashed).map_err(|_| HashError::VerifyFailed)
        })
        .await
        .map_err(|_| HashError::TaskFailed)?
    }
}

#[cfg(test)]
mod tests {
    use super::BcryptHasher;
    use crate::auth::application::ports::outgoing::password_hasher::{HashError, PasswordHasher};

    #[tokio::test]
    async fn test_bcrypt_hash_and_verify_password() {
        let hasher = BcryptHasher::new();
        let password = "SecurePassword123";

        let hashed_password = hasher.hash_password(password).await;
        assert!(hashed_password.is_ok());

        let hashed_password = hashed_password.unwrap();

        let verify_correct = hasher.verify_password(password, &hashed_password).await;
        assert!(verify_correct.is_ok());
        assert!(verify_correct.unwrap());

        let verify_wrong = hasher
            .verify_password("WrongPassword", &hashed_password)
            .await;
        assert!(verify_wrong.is_ok());
        assert!(!verify_wrong.unwrap());

        let verify_invalid_hash = hasher.verify_password(password, "invalid-hash").await;
        assert!(matches!(verify_invalid_hash, Err(HashError::VerifyFailed)));
    }
}
