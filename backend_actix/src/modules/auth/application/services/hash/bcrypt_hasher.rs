use super::password_hasher::PasswordHasher;
use bcrypt::{hash, verify, DEFAULT_COST};

pub struct BcryptHasher;

impl PasswordHasher for BcryptHasher {
    fn hash_password(&self, password: &str) -> Result<String, String> {
        hash(password, DEFAULT_COST).map_err(|e| e.to_string()) // Ensure it returns Result
    }

    fn verify_password(&self, password: &str, hashed: &str) -> Result<bool, String> {
        verify(password, hashed).map_err(|e| e.to_string()) // Ensure it returns Result
    }
}

#[cfg(test)]
mod tests {
    use super::BcryptHasher;
    use super::*;

    #[test]
    fn test_bcrypt_hash_and_verify_password() {
        let hasher = BcryptHasher;
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
