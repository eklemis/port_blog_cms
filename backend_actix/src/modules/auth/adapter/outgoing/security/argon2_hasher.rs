use argon2::{
    password_hash::{
        Error as PasswordHashError, PasswordHash, PasswordHasher as _, PasswordVerifier, SaltString,
    },
    Argon2,
};
use rand_core::OsRng;

use crate::auth::application::ports::outgoing::password_hasher::{
    HashError, PasswordHasher as HasherTrait,
};

#[derive(Clone)]
pub struct Argon2Hasher {
    argon2: Argon2<'static>,
    salt_override: Option<SaltString>,
}

impl Argon2Hasher {
    pub fn new() -> Self {
        Self {
            argon2: Argon2::default(),
            salt_override: None,
        }
    }

    #[cfg(test)]
    pub fn with_salt(salt: SaltString) -> Self {
        Self {
            argon2: Argon2::default(),
            salt_override: Some(salt),
        }
    }
}

impl HasherTrait for Argon2Hasher {
    fn hash_password(&self, password: &str) -> Result<String, HashError> {
        let salt = match &self.salt_override {
            Some(s) => s.clone(),
            None => SaltString::generate(&mut OsRng),
        };

        self.argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|_| HashError::HashFailed)
    }

    fn verify_password(&self, password: &str, hashed: &str) -> Result<bool, HashError> {
        let parsed_hash = PasswordHash::new(hashed).map_err(|_| HashError::VerifyFailed)?;

        match self
            .argon2
            .verify_password(password.as_bytes(), &parsed_hash)
        {
            Ok(_) => Ok(true),
            Err(PasswordHashError::Password) => Ok(false), // mismatch is NOT an error
            Err(_) => Err(HashError::VerifyFailed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Argon2Hasher;
    use super::*;

    #[test]
    fn test_argon2_hash_and_verify_password() {
        let hasher = Argon2Hasher::new();
        let password = "SecurePassword123";

        // Hash the password
        let hashed_password = hasher.hash_password(password);
        assert!(
            hashed_password.is_ok(),
            "Expected password hashing to succeed"
        );

        let hashed_password = hashed_password.unwrap();

        // Verify the correct password
        let verify_correct = hasher.verify_password(password, &hashed_password);
        assert!(
            verify_correct.is_ok(),
            "Expected password verification to succeed"
        );
        assert!(verify_correct.unwrap(), "Password should match");

        // Verify an incorrect password
        let verify_wrong = hasher.verify_password("WrongPassword", &hashed_password);
        assert!(
            verify_wrong.is_ok(),
            "Expected password verification to succeed"
        );
        assert!(!verify_wrong.unwrap(), "Password should not match");

        // Verify invalid hash
        let verify_invalid_hash = hasher.verify_password(password, "invalid-hash");
        assert!(
            verify_invalid_hash.is_err(),
            "Expected error for invalid hash format"
        );
    }
    #[test]
    fn test_hash_password_error() {
        let bad_salt_bytes = b"short";
        let bad_salt = SaltString::encode_b64(bad_salt_bytes).unwrap();

        let hasher = Argon2Hasher::with_salt(bad_salt);
        let result = hasher.hash_password("abc123");

        assert!(matches!(result, Err(HashError::HashFailed)));
    }

    #[test]
    fn test_verify_password_error_branch() {
        let hasher = Argon2Hasher::new();

        let valid_hash = hasher.hash_password("password123").unwrap();

        let mut parts: Vec<&str> = valid_hash.split('$').collect();
        parts[3] = "m=0,t=0,p=0";

        let tampered_hash = parts.join("$");

        let result = hasher.verify_password("password123", &tampered_hash);

        assert!(matches!(result, Err(HashError::VerifyFailed)));
    }
}
