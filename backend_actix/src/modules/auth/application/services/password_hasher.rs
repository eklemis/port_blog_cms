pub trait PasswordHasher: Send + Sync {
    fn hash_password(&self, password: &str) -> Result<String, String>;
    fn verify_password(&self, password: &str, hashed: &str) -> Result<bool, String>;
}
