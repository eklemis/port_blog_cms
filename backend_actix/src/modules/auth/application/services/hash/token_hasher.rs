use sha2::{Digest, Sha256};

/// Hash a token using SHA-256 for storage
/// Never store raw tokens in the database!
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_token_consistency() {
        let token = "my_token_123";
        let hash1 = hash_token(token);
        let hash2 = hash_token(token);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_token_different_inputs() {
        let token1 = "token_1";
        let token2 = "token_2";

        let hash1 = hash_token(token1);
        let hash2 = hash_token(token2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_token_length() {
        let token = "any_token";
        let hash = hash_token(token);

        // SHA-256 produces 64 hex characters
        assert_eq!(hash.len(), 64);
    }
}
