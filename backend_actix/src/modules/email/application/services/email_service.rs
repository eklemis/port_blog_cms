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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use mockall::{mock, predicate::*};
    use std::sync::Arc;

    // Mock EmailSender trait
    mock! {
        pub EmailSenderMock {}
        #[async_trait]
        impl EmailSender for EmailSenderMock {
            async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), String>;
        }
    }

    #[test]
    fn test_email_service_debug_format() {
        // Create a mock EmailSender
        let mock_sender = Arc::new(MockEmailSenderMock::new()) as Arc<dyn EmailSender + Send + Sync>;

        // Create EmailService instance
        let email_service = EmailService::new(mock_sender);

        // Format using Debug
        let debug_output = format!("{:?}", email_service);

        // Verify the output matches the expected Debug format
        assert_eq!(
            debug_output,
            "EmailService { sender: \"<dyn EmailSender>\" }",
            "Unexpected Debug output: got {}", debug_output
        );
    }
}