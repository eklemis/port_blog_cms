use std::env;

/// See the module documentation.
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// HS256 signing key. At least 32 bytes, enforced at startup.
    pub secret_key: String,
    /// The `iss` claim stamped on every token.
    pub issuer: String,
    /// Access-token lifetime in seconds. Validated to 1..=86400.
    pub access_token_expiry: i64,
    /// Refresh-token lifetime in seconds. Must exceed the access lifetime.
    pub refresh_token_expiry: i64,
    /// Verification-link lifetime in seconds.
    pub verification_token_expiry: i64,
    /// Shorter than verification by default: a reset link is a live credential
    /// for the account, so its useful lifetime should be the time a person
    /// needs to read one email, not a day.
    pub password_reset_expiry: i64, // Expiration in seconds
}

impl JwtConfig {
    /// Helper function to parse expiry values
    fn parse_expiry(key: &str, default: &str) -> i64 {
        env::var(key)
            .unwrap_or_else(|_| default.to_string())
            .parse::<i64>()
            .unwrap_or_else(|_| panic!("Invalid {} value", key))
    }
    /// Load JWT configuration from environment variables.
    ///
    /// Deliberately does not call `dotenvy` itself. `main.rs` already loads the
    /// env file before constructing this, so the call was redundant in
    /// production — and it made the type impossible to test, because clearing a
    /// variable did nothing once `.env` was re-read underneath. Loading the
    /// environment is the entry point's job, not a config struct's.
    pub fn from_env() -> Self {
        let secret_key = env::var("JWT_SECRET").expect("JWT_SECRET must be set");

        // Validate secret key length (HS256 requires at least 32 bytes)
        if secret_key.len() < 32 {
            panic!("JWT_SECRET must be at least 32 characters long for HS256 algorithm");
        }

        let access_token_expiry = Self::parse_expiry("JWT_ACCESS_EXPIRY", "1800");
        let refresh_token_expiry = Self::parse_expiry("JWT_REFRESH_EXPIRY", "604800");
        let verification_token_expiry = Self::parse_expiry("JWT_VERIFICATION_EXPIRY", "86400");
        let password_reset_expiry = Self::parse_expiry("JWT_PASSWORD_RESET_EXPIRY", "3600");

        // Validate expiry values
        if access_token_expiry <= 0 || access_token_expiry > 86400 {
            panic!("JWT_ACCESS_EXPIRY must be between 1 and 86400 seconds (24 hours)");
        }

        if refresh_token_expiry <= access_token_expiry {
            panic!("JWT_REFRESH_EXPIRY must be greater than JWT_ACCESS_EXPIRY");
        }

        let issuer = env::var("JWT_ISSUER").unwrap_or_else(|_| "Ekstion".to_string());

        Self {
            secret_key,
            issuer,
            access_token_expiry,
            refresh_token_expiry,
            verification_token_expiry,
            password_reset_expiry,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// `from_env` reads process-global state, so these tests must not run
    /// concurrently with each other. The suite already sets RUST_TEST_THREADS=1,
    /// but this guard makes the requirement explicit rather than inherited, and
    /// keeps the tests correct if that ever changes.
    static ENV_GUARD: Mutex<()> = Mutex::new(());

    const KEYS: &[&str] = &[
        "JWT_SECRET",
        "JWT_ISSUER",
        "JWT_ACCESS_EXPIRY",
        "JWT_REFRESH_EXPIRY",
        "JWT_VERIFICATION_EXPIRY",
        "JWT_PASSWORD_RESET_EXPIRY",
    ];

    /// Saves the JWT variables, applies an override set, and restores on drop.
    ///
    /// Restoring matters: other tests call `dotenvy`, and a leaked value here
    /// would follow them into an unrelated failure.
    struct EnvScope {
        saved: Vec<(String, Option<String>)>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl EnvScope {
        fn new(overrides: &[(&str, &str)]) -> Self {
            let lock = ENV_GUARD.lock().unwrap_or_else(|e| e.into_inner());
            let saved = KEYS
                .iter()
                .map(|k| (k.to_string(), env::var(k).ok()))
                .collect();

            for k in KEYS {
                env::remove_var(k);
            }
            for (k, v) in overrides {
                env::set_var(k, v);
            }

            Self { saved, _lock: lock }
        }
    }

    impl Drop for EnvScope {
        fn drop(&mut self) {
            for (k, v) in &self.saved {
                match v {
                    Some(val) => env::set_var(k, val),
                    None => env::remove_var(k),
                }
            }
        }
    }

    const VALID_SECRET: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn reads_every_value_from_the_environment() {
        let _scope = EnvScope::new(&[
            ("JWT_SECRET", VALID_SECRET),
            ("JWT_ISSUER", "TestIssuer"),
            ("JWT_ACCESS_EXPIRY", "900"),
            ("JWT_REFRESH_EXPIRY", "7200"),
            ("JWT_VERIFICATION_EXPIRY", "1200"),
            ("JWT_PASSWORD_RESET_EXPIRY", "600"),
        ]);

        let c = JwtConfig::from_env();
        assert_eq!(c.secret_key, VALID_SECRET);
        assert_eq!(c.issuer, "TestIssuer");
        assert_eq!(c.access_token_expiry, 900);
        assert_eq!(c.refresh_token_expiry, 7200);
        assert_eq!(c.verification_token_expiry, 1200);
        assert_eq!(c.password_reset_expiry, 600);
    }

    #[test]
    fn falls_back_to_defaults_when_only_the_secret_is_set() {
        let _scope = EnvScope::new(&[("JWT_SECRET", VALID_SECRET)]);

        let c = JwtConfig::from_env();
        assert_eq!(c.issuer, "Ekstion");
        assert_eq!(c.access_token_expiry, 1800);
        assert_eq!(c.refresh_token_expiry, 604800);
        assert_eq!(c.verification_token_expiry, 86400);
        // Shorter than verification by design: a reset link is a live
        // credential for the account.
        assert_eq!(c.password_reset_expiry, 3600);
    }

    /// HS256 needs at least 32 bytes of key. A shorter secret is a silent
    /// weakening of every token the service issues, so it fails loudly.
    #[test]
    #[should_panic(expected = "at least 32 characters")]
    fn rejects_a_secret_shorter_than_the_hs256_minimum() {
        let _scope = EnvScope::new(&[("JWT_SECRET", "too-short")]);
        JwtConfig::from_env();
    }

    #[test]
    #[should_panic(expected = "JWT_SECRET must be set")]
    fn requires_a_secret() {
        let _scope = EnvScope::new(&[]);
        JwtConfig::from_env();
    }

    #[test]
    #[should_panic(expected = "Invalid JWT_ACCESS_EXPIRY")]
    fn rejects_a_non_numeric_expiry() {
        let _scope = EnvScope::new(&[
            ("JWT_SECRET", VALID_SECRET),
            ("JWT_ACCESS_EXPIRY", "not-a-number"),
        ]);
        JwtConfig::from_env();
    }

    #[test]
    #[should_panic(expected = "between 1 and 86400")]
    fn rejects_a_zero_access_expiry() {
        let _scope = EnvScope::new(&[("JWT_SECRET", VALID_SECRET), ("JWT_ACCESS_EXPIRY", "0")]);
        JwtConfig::from_env();
    }

    /// An access token good for more than a day defeats the point of pairing it
    /// with a refresh token.
    #[test]
    #[should_panic(expected = "between 1 and 86400")]
    fn rejects_an_access_expiry_beyond_a_day() {
        let _scope = EnvScope::new(&[("JWT_SECRET", VALID_SECRET), ("JWT_ACCESS_EXPIRY", "86401")]);
        JwtConfig::from_env();
    }

    /// A refresh token that expires no later than the access token cannot
    /// refresh anything.
    #[test]
    #[should_panic(expected = "greater than JWT_ACCESS_EXPIRY")]
    fn rejects_a_refresh_expiry_not_longer_than_the_access_expiry() {
        let _scope = EnvScope::new(&[
            ("JWT_SECRET", VALID_SECRET),
            ("JWT_ACCESS_EXPIRY", "3600"),
            ("JWT_REFRESH_EXPIRY", "3600"),
        ]);
        JwtConfig::from_env();
    }
}
