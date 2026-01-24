#[derive(Debug, Clone, thiserror::Error)]
pub enum HashError {
    #[error("Password hashing failed")]
    HashFailed,

    #[error("Password verification failed")]
    VerifyFailed,
}

pub trait PasswordHasher: Send + Sync {
    fn hash_password(&self, password: &str) -> Result<String, HashError>;
    fn verify_password(&self, password: &str, hash: &str) -> Result<bool, HashError>;
}
