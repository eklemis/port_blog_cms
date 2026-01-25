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
    #[cfg(test)]
    salt_override: Option<SaltString>,
}

impl Argon2Hasher {
    pub fn new() -> Self {
        // Budget VPS friendly: 4MB memory, 3 iterations, 1 thread
        let params = Params::new(4 * 1024, 3, 1, None).expect("Invalid Argon2 params");

        Self {
            params,
            #[cfg(test)]
            salt_override: None,
        }
    }
    /// Create with custom params (for testing or different environments)
    pub fn with_params(memory_kib: u32, iterations: u32, parallelism: u32) -> Self {
        let params =
            Params::new(memory_kib, iterations, parallelism, None).expect("Invalid Argon2 params");

        Self {
            params,
            #[cfg(test)]
            salt_override: None,
        }
    }
    /// Environment-based configuration
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

        Self::with_params(memory_kib, iterations, parallelism)
    }
    #[cfg(test)]
    pub fn with_fixed_salt(salt: &str) -> Self {
        Self {
            params: Params::new(4 * 1024, 3, 1, None).expect("Invalid params"),
            salt_override: Some(SaltString::from_b64(salt).expect("Invalid salt")),
        }
    }
}

#[async_trait]
impl HasherTrait for Argon2Hasher {
    async fn hash_password(&self, password: &str) -> Result<String, HashError> {
        let password = password.to_string();
        let params = self.params.clone();

        #[cfg(test)]
        let salt_override = self.salt_override.clone();

        tokio::task::spawn_blocking(move || {
            let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

            #[cfg(test)]
            let salt = salt_override.unwrap_or_else(|| SaltString::generate(&mut OsRng));

            #[cfg(not(test))]
            let salt = SaltString::generate(&mut OsRng);

            argon2
                .hash_password(password.as_bytes(), &salt)
                .map(|hash| hash.to_string())
                .map_err(|_| HashError::HashFailed)
        })
        .await
        .map_err(|_| HashError::TaskFailed)?
    }

    async fn verify_password(&self, password: &str, hash: &str) -> Result<bool, HashError> {
        let password = password.to_string();
        let hash = hash.to_string();

        tokio::task::spawn_blocking(move || {
            let parsed_hash = PasswordHash::new(&hash).map_err(|_| HashError::VerifyFailed)?;

            match Argon2::default().verify_password(password.as_bytes(), &parsed_hash) {
                Ok(_) => Ok(true),
                Err(PasswordHashError::Password) => Ok(false),
                Err(_) => Err(HashError::VerifyFailed),
            }
        })
        .await
        .map_err(|_| HashError::TaskFailed)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_argon2_hash_and_verify_password() {
        let hasher = Argon2Hasher::new();
        let password = "SecurePassword123";

        // Hash the password
        let hashed_password = hasher.hash_password(password).await;
        assert!(
            hashed_password.is_ok(),
            "Expected password hashing to succeed"
        );

        let hashed_password = hashed_password.unwrap();

        // Verify the correct password
        let verify_correct = hasher.verify_password(password, &hashed_password).await;
        assert!(
            verify_correct.is_ok(),
            "Expected password verification to succeed"
        );
        assert!(verify_correct.unwrap(), "Password should match");

        // Verify an incorrect password
        let verify_wrong = hasher
            .verify_password("WrongPassword", &hashed_password)
            .await;
        assert!(
            verify_wrong.is_ok(),
            "Expected password verification to succeed"
        );
        assert!(!verify_wrong.unwrap(), "Password should not match");

        // Verify invalid hash
        let verify_invalid_hash = hasher.verify_password(password, "invalid-hash").await;
        assert!(
            verify_invalid_hash.is_err(),
            "Expected error for invalid hash format"
        );
    }

    #[tokio::test]
    async fn test_hash_password_error() {
        let bad_salt_bytes = b"short";
        let bad_salt = SaltString::encode_b64(bad_salt_bytes).unwrap();

        let hasher = Argon2Hasher::with_fixed_salt(bad_salt.as_str());
        let result = hasher.hash_password("abc123").await;

        assert!(matches!(result, Err(HashError::HashFailed)));
    }

    #[tokio::test]
    async fn test_verify_password_error_branch() {
        let hasher = Argon2Hasher::new();

        let valid_hash = hasher.hash_password("password123").await.unwrap();

        let mut parts: Vec<&str> = valid_hash.split('$').collect();
        parts[3] = "m=0,t=0,p=0";

        let tampered_hash = parts.join("$");

        let result = hasher.verify_password("password123", &tampered_hash).await;

        assert!(matches!(result, Err(HashError::VerifyFailed)));
    }
}
