use argon2::{
    password_hash::{
        Error as PasswordHashError, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
    Argon2,
};
use rand_core::OsRng;

use super::password_hasher::PasswordHasher as HasherTrait;

pub struct Argon2Hasher;

impl HasherTrait for Argon2Hasher {
    fn hash_password(&self, password: &str) -> Result<String, String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        match argon2.hash_password(password.as_bytes(), &salt) {
            Ok(password_hash) => Ok(password_hash.to_string()),
            Err(e) => Err(format!("Failed to hash password: {}", e)),
        }
    }

    fn verify_password(&self, password: &str, hashed: &str) -> Result<bool, String> {
        match PasswordHash::new(hashed) {
            Ok(parsed_hash) => {
                match Argon2::default().verify_password(password.as_bytes(), &parsed_hash) {
                    Ok(_) => Ok(true),
                    Err(PasswordHashError::Password) => Ok(false),
                    Err(e) => Err(format!("Password verification failed: {}", e)),
                }
            }
            Err(_) => Err("Invalid hash format".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Argon2Hasher;
    use super::*;

    #[test]
    fn test_argon2_hash_and_verify_password() {
        let hasher = Argon2Hasher;
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
}
