#[cfg(test)]
pub mod test_helpers {
    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};

    pub fn create_test_jwt_service() -> JwtTokenService {
        let jwt_config = JwtConfig {
            issuer: "Ekstion".to_string(),
            secret_key: std::env::var("TEST_JWT_SECRET")
                .unwrap_or_else(|_| "FAKE_JWT_SECRET_DO_NOT_USE".to_string()),

            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
        };
        JwtTokenService::new(jwt_config)
    }
}
