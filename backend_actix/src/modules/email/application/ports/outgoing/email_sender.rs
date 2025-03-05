use async_trait::async_trait;

#[async_trait]
pub trait EmailSender: Send + Sync {
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), String>;
}
