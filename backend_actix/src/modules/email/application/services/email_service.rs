use crate::email::application::ports::outgoing::email_sender::EmailSender;
use std::fmt;
use std::sync::Arc;

#[derive(Clone)]
pub struct EmailService {
    sender: Arc<dyn EmailSender + Send + Sync>,
}

impl fmt::Debug for EmailService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EmailService")
            .field("sender", &"<dyn EmailSender>")
            .finish()
    }
}

impl EmailService {
    pub fn new(sender: Arc<dyn EmailSender + Send + Sync>) -> Self {
        Self { sender }
    }

    pub async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
        self.sender.send_email(to, subject, body).await
    }
}
