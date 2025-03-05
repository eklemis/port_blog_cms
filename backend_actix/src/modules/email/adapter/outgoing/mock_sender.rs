use crate::email::application::ports::outgoing::email_sender::EmailSender;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

pub struct MockEmailSender {
    sent_emails: Arc<Mutex<Vec<(String, String, String)>>>, // Stores sent emails for verification
}

impl MockEmailSender {
    pub fn new() -> Self {
        Self {
            sent_emails: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get_sent_emails(&self) -> Vec<(String, String, String)> {
        self.sent_emails.lock().unwrap().clone()
    }
}

#[async_trait]
impl EmailSender for MockEmailSender {
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
        self.sent_emails.lock().unwrap().push((
            to.to_string(),
            subject.to_string(),
            body.to_string(),
        ));
        Ok(())
    }
}
