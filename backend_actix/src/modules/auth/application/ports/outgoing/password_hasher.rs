use async_trait::async_trait;

#[derive(Debug, Clone, thiserror::Error)]
pub enum HashError {
    #[error("Password hashing failed")]
    HashFailed,

    #[error("Password verification failed")]
    VerifyFailed,

    #[error("Background task failed")]
    TaskFailed,
}

#[async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash_password(&self, password: &str) -> Result<String, HashError>;
    async fn verify_password(&self, password: &str, hash: &str) -> Result<bool, HashError>;
}
