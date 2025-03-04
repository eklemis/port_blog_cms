use super::{
    argon2_hasher::Argon2Hasher, bcrypt_hasher::BcryptHasher, password_hasher::PasswordHasher,
};
use std::sync::Arc;
use tokio::task;

pub enum HashingAlgorithm {
    Argon2,
    Bcrypt,
}

pub struct PasswordHashingService {
    hasher: Arc<dyn PasswordHasher + Send + Sync>,
}

impl Clone for PasswordHashingService {
    fn clone(&self) -> Self {
        Self {
            hasher: Arc::clone(&self.hasher),
        }
    }
}

impl PasswordHashingService {
    pub fn new(algorithm: HashingAlgorithm) -> Self {
        let hasher: Arc<dyn PasswordHasher + Send + Sync> = match algorithm {
            HashingAlgorithm::Argon2 => Arc::new(Argon2Hasher),
            HashingAlgorithm::Bcrypt => Arc::new(BcryptHasher),
        };
        Self { hasher }
    }

    pub async fn hash_password(&self, password: String) -> Result<String, String> {
        let hasher = Arc::clone(&self.hasher);
        task::spawn_blocking(move || hasher.hash_password(&password))
            .await
            .map_err(|e| e.to_string())? // Handle tokio task error
            .map_err(|e| e.to_string()) // Handle password hashing error
    }

    pub async fn verify_password(&self, password: String, hash: String) -> Result<bool, String> {
        let hasher = Arc::clone(&self.hasher);
        task::spawn_blocking(move || hasher.verify_password(&password, &hash))
            .await
            .map_err(|e| e.to_string())? // Handle tokio task error
            .map_err(|e| e.to_string()) // Handle password verification error
    }
}

#[cfg(test)]
mod tests {
    use super::{HashingAlgorithm, PasswordHashingService};
    use tokio::test;

    #[test]
    async fn test_password_hashing_service_with_argon2() {
        let service = PasswordHashingService::new(HashingAlgorithm::Argon2);
        let password = "SecurePassword123";

        let hashed_password = service.hash_password(password.to_owned()).await;
        assert!(
            hashed_password.is_ok(),
            "Expected password hashing to succeed"
        );

        let hashed_password = hashed_password.unwrap();

        let verify_correct = service
            .verify_password(password.to_owned(), hashed_password.clone())
            .await;
        assert!(
            verify_correct.is_ok(),
            "Expected password verification to succeed"
        );
        assert!(verify_correct.unwrap(), "Password should match");

        let verify_wrong = service
            .verify_password(String::from("WrongPassword"), hashed_password.clone())
            .await;
        assert!(
            verify_wrong.is_ok(),
            "Expected password verification to succeed"
        );
        assert!(!verify_wrong.unwrap(), "Password should not match");
    }

    #[test]
    async fn test_password_hashing_service_with_bcrypt() {
        let service = PasswordHashingService::new(HashingAlgorithm::Bcrypt);
        let password = "SecurePassword123";

        let hashed_password = service.hash_password(password.to_owned()).await;
        assert!(
            hashed_password.is_ok(),
            "Expected password hashing to succeed"
        );

        let hashed_password = hashed_password.unwrap();

        let verify_correct = service
            .verify_password(password.to_owned(), hashed_password.clone())
            .await;
        assert!(
            verify_correct.is_ok(),
            "Expected password verification to succeed"
        );
        assert!(verify_correct.unwrap(), "Password should match");

        let verify_wrong = service
            .verify_password(String::from("WrongPassword"), hashed_password.clone())
            .await;
        assert!(
            verify_wrong.is_ok(),
            "Expected password verification to succeed"
        );
        assert!(!verify_wrong.unwrap(), "Password should not match");
    }
}
