use bcrypt::{hash, verify, DEFAULT_COST};

use crate::auth::application::ports::outgoing::password_hasher::{HashError, PasswordHasher};

pub struct BcryptHasher;

impl PasswordHasher for BcryptHasher {
    fn hash_password(&self, password: &str) -> Result<String, HashError> {
        hash(password, DEFAULT_COST).map_err(|_| HashError::HashFailed)
    }

    fn verify_password(&self, password: &str, hashed: &str) -> Result<bool, HashError> {
        verify(password, hashed).map_err(|_| HashError::VerifyFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::BcryptHasher;
    use crate::auth::application::ports::outgoing::password_hasher::{HashError, PasswordHasher};

    #[test]
    fn test_bcrypt_hash_and_verify_password() {
        let hasher = BcryptHasher;
        let password = "SecurePassword123";

        let hashed_password = hasher.hash_password(password);
        assert!(hashed_password.is_ok());

        let hashed_password = hashed_password.unwrap();

        let verify_correct = hasher.verify_password(password, &hashed_password);
        assert!(verify_correct.is_ok());
        assert!(verify_correct.unwrap());

        let verify_wrong = hasher.verify_password("WrongPassword", &hashed_password);
        assert!(verify_wrong.is_ok());
        assert!(!verify_wrong.unwrap());

        let verify_invalid_hash = hasher.verify_password(password, "invalid-hash");
        assert!(matches!(verify_invalid_hash, Err(HashError::VerifyFailed)));
    }
}
