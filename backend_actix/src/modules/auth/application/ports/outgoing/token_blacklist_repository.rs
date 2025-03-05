use async_trait::async_trait;

#[async_trait]
pub trait TokenBlacklistRepository {
    async fn blacklist_token(&self, token: &str) -> Result<(), String>;
    async fn is_token_blacklisted(&self, token: &str) -> Result<bool, String>;
}
