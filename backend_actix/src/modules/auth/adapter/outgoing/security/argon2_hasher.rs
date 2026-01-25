use argon2::{
    password_hash::{
        Error as PasswordHashError, PasswordHash, PasswordHasher as _, PasswordVerifier, SaltString,
    },
    Algorithm, Argon2, Params, Version,
};
use async_trait::async_trait;
use rand_core::OsRng;

use crate::auth::application::ports::outgoing::password_hasher::{
    HashError, PasswordHasher as HasherTrait,
};

#[derive(Clone)]
pub struct Argon2Hasher {
    params: Params,
    use_blocking_task: bool,
    #[cfg(test)]
    salt_override: Option<SaltString>,
}

impl Argon2Hasher {
    /// Create with default settings (auto-detects environment)
    pub fn new() -> Self {
        Self::from_env()
    }
    /// Create from environment variables
    ///
    /// Environment variables:
    /// - ARGON2_MEMORY_KIB: Memory cost in KiB (default: 4096)
    /// - ARGON2_ITERATIONS: Time cost (default: 3)
    /// - ARGON2_PARALLELISM: Parallelism factor (default: 1)
    /// - USE_BLOCKING_HASH: Whether to use spawn_blocking (default: false)
    pub fn from_env() -> Self {
        let memory_kib: u32 = std::env::var("ARGON2_MEMORY_KIB")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(4 * 1024); // 4MB default

        let iterations: u32 = std::env::var("ARGON2_ITERATIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);

        let parallelism: u32 = std::env::var("ARGON2_PARALLELISM")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);

        let use_blocking_task: bool = std::env::var("USE_BLOCKING_HASH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false);

        Self::with_config(memory_kib, iterations, parallelism, use_blocking_task)
    }
    /// Create with explicit configuration
    pub fn with_config(
        memory_kib: u32,
        iterations: u32,
        parallelism: u32,
        use_blocking_task: bool,
    ) -> Self {
        let params =
            Params::new(memory_kib, iterations, parallelism, None).expect("Invalid Argon2 params");

        Self {
            params,
            use_blocking_task,
            #[cfg(test)]
            salt_override: None,
        }
    }
    /// Create optimized for fast environments (M1 Mac, good servers)
    pub fn fast_env() -> Self {
        Self::with_config(
            4 * 1024, // 4MB
            3,        // iterations
            1,        // parallelism
            false,    // no spawn_blocking
        )
    }
    /// Create optimized for budget VPS (slow CPU, limited memory)
    pub fn budget_vps() -> Self {
        Self::with_config(
            4 * 1024, // 4MB
            3,        // iterations
            1,        // parallelism
            true,     // use spawn_blocking to prevent async runtime starvation
        )
    }
    /// Create with fixed salt for testing (deterministic hashes)
    #[cfg(test)]
    pub fn with_fixed_salt(salt: &str) -> Self {
        Self {
            params: Params::new(4 * 1024, 3, 1, None).expect("Invalid params"),
            use_blocking_task: false,
            salt_override: Some(SaltString::from_b64(salt).expect("Invalid salt")),
        }
    }
    /// Build Argon2 instance with current params
    fn build_argon2(&self) -> Argon2<'_> {
        Argon2::new(Algorithm::Argon2id, Version::V0x13, self.params.clone())
    }
    /// Synchronous hash implementation
    fn hash_sync(
        password: &str,
        params: Params,
        salt: Option<SaltString>,
    ) -> Result<String, HashError> {
        let salt = salt.unwrap_or_else(|| SaltString::generate(&mut OsRng));
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|_| HashError::HashFailed)
    }
    /// Synchronous verify implementation
    fn verify_sync(password: &str, hash: &str) -> Result<bool, HashError> {
        let parsed_hash = PasswordHash::new(hash).map_err(|_| HashError::VerifyFailed)?;

        match Argon2::default().verify_password(password.as_bytes(), &parsed_hash) {
            Ok(_) => Ok(true),
            Err(PasswordHashError::Password) => Ok(false),
            Err(_) => Err(HashError::VerifyFailed),
        }
    }
}

impl Default for Argon2Hasher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HasherTrait for Argon2Hasher {
    async fn hash_password(&self, password: &str) -> Result<String, HashError> {
        let params = self.params.clone();

        #[cfg(test)]
        let salt = self.salt_override.clone();
        #[cfg(not(test))]
        let salt: Option<SaltString> = None;

        if self.use_blocking_task {
            // For slow environments - offload to blocking thread pool
            let password = password.to_string();

            tokio::task::spawn_blocking(move || Self::hash_sync(&password, params, salt))
                .await
                .map_err(|_| HashError::TaskFailed)?
        } else {
            // For fast environments - direct execution
            Self::hash_sync(password, params, salt)
        }
    }

    async fn verify_password(&self, password: &str, hash: &str) -> Result<bool, HashError> {
        if self.use_blocking_task {
            // For slow environments - offload to blocking thread pool
            let password = password.to_string();
            let hash = hash.to_string();

            tokio::task::spawn_blocking(move || Self::verify_sync(&password, &hash))
                .await
                .map_err(|_| HashError::TaskFailed)?
        } else {
            // For fast environments - direct execution
            Self::verify_sync(password, hash)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hash_and_verify_password() {
        let hasher = Argon2Hasher::fast_env();
        let password = "SecurePassword123";

        let hashed = hasher.hash_password(password).await;
        assert!(hashed.is_ok());

        let hashed = hashed.unwrap();

        // Verify correct password
        let verify_correct = hasher.verify_password(password, &hashed).await;
        assert!(verify_correct.is_ok());
        assert!(verify_correct.unwrap());

        // Verify incorrect password
        let verify_wrong = hasher.verify_password("WrongPassword", &hashed).await;
        assert!(verify_wrong.is_ok());
        assert!(!verify_wrong.unwrap());

        // Verify invalid hash
        let verify_invalid = hasher.verify_password(password, "invalid-hash").await;
        assert!(verify_invalid.is_err());
    }

    #[tokio::test]
    async fn test_hash_and_verify_with_blocking() {
        let hasher = Argon2Hasher::budget_vps();
        let password = "SecurePassword123";

        let hashed = hasher.hash_password(password).await.unwrap();

        let is_valid = hasher.verify_password(password, &hashed).await.unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_fixed_salt_produces_deterministic_hash() {
        let salt = "c29tZXNhbHQ"; // "somesalt" in base64
        let hasher = Argon2Hasher::with_fixed_salt(salt);
        let password = "test123";

        let hash1 = hasher.hash_password(password).await.unwrap();
        let hash2 = hasher.hash_password(password).await.unwrap();

        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_hash_password_error() {
        let bad_salt_bytes = b"short";
        let bad_salt = SaltString::encode_b64(bad_salt_bytes).unwrap();

        let hasher = Argon2Hasher::with_fixed_salt(bad_salt.as_str());
        let result = hasher.hash_password("abc123").await;

        assert!(matches!(result, Err(HashError::HashFailed)));
    }
}
