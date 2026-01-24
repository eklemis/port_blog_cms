use crate::email::application::ports::outgoing::email_sender::EmailSender;
use async_trait::async_trait;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{
    message::header::ContentType, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

#[async_trait]
pub trait Mailer: Send + Sync {
    async fn send(&self, email: Message) -> Result<(), String>;
}

pub struct SmtpEmailSender {
    mailer: Box<dyn Mailer>,
    from_email: String,
}

#[async_trait]
impl Mailer for AsyncSmtpTransport<Tokio1Executor> {
    async fn send(&self, email: Message) -> Result<(), String> {
        AsyncTransport::send(self, email)
            .await
            .map(|_resp| ())
            .map_err(|e| e.to_string())
    }
}

impl SmtpEmailSender {
    pub fn new_with_mailer(mailer: Box<dyn Mailer>, from_email: &str) -> Self {
        Self {
            mailer,
            from_email: from_email.to_string(),
        }
    }
    pub fn new(
        smtp_server: &str,
        smtp_username: &str,
        smtp_password: &str,
        from_email: &str,
    ) -> Self {
        let creds = Credentials::new(smtp_username.to_string(), smtp_password.to_string());

        let transport = AsyncSmtpTransport::<Tokio1Executor>::relay(smtp_server)
            .unwrap()
            .credentials(creds)
            .build();

        let mailer: Box<dyn Mailer> = Box::new(transport);

        Self {
            mailer,
            from_email: from_email.to_string(),
        }
    }
    // Local/test constructor (Mailpit, MailHog, etc.)
    pub fn new_local(host: &str, port: u16, from_email: &str) -> Self {
        let transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
            .port(port)
            .build();

        Self {
            mailer: Box::new(transport),
            from_email: from_email.to_string(),
        }
    }
}

#[async_trait]
impl EmailSender for SmtpEmailSender {
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
        let email = Message::builder()
            .from(self.from_email.parse().map_err(|e| format!("{:?}", e))?)
            .to(to.parse().map_err(|e| format!("{:?}", e))?)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(body.to_string())
            .map_err(|e| e.to_string())?;

        self.mailer.send(email).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    struct MockMailer;
    #[async_trait]
    impl Mailer for MockMailer {
        async fn send(&self, _email: Message) -> Result<(), String> {
            Ok(()) // always succeed
        }
    }

    #[tokio::test]
    async fn test_send_email_success_unit() {
        let sender = SmtpEmailSender::new_with_mailer(Box::new(MockMailer), "sender@example.com");

        let result = sender
            .send_email("recipient@example.com", "Test", "<p>Unit test</p>")
            .await;

        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
    }

    #[tokio::test]
    async fn test_send_email_invalid_from_address() {
        struct DummyMailer;
        #[async_trait]
        impl Mailer for DummyMailer {
            async fn send(&self, _: Message) -> Result<(), String> {
                panic!("Should not reach mailer. Send if email is invalid");
            }
        }

        let sender = SmtpEmailSender::new_with_mailer(
            Box::new(DummyMailer),
            "invalid-from-email", // Not a valid email address
        );

        let result = sender
            .send_email("recipient@example.com", "Subject", "<p>Test</p>")
            .await;

        assert!(
            result.is_err(),
            "Expected error from invalid 'from' address"
        );

        if let Err(err) = result {
            assert!(
                err.contains("invalid") || err.contains("Invalid") || err.contains("@"),
                "Unexpected error format: {}",
                err
            );
        }
    }
    #[tokio::test]
    async fn test_send_email_invalid_to_address() {
        struct DummyMailer;
        #[async_trait]
        impl Mailer for DummyMailer {
            async fn send(&self, _: Message) -> Result<(), String> {
                panic!("Should not reach mailer. Send if 'to' is invalid");
            }
        }

        let sender = SmtpEmailSender::new_with_mailer(Box::new(DummyMailer), "sender@example.com");

        let result = sender
            .send_email("not-an-email", "Subject", "<p>Test</p>")
            .await;

        assert!(result.is_err(), "Expected error from invalid 'to' address");

        if let Err(err) = result {
            assert!(
                err.contains("invalid") || err.contains("Invalid") || err.contains("@"),
                "Unexpected error format: {}",
                err
            );
        }
    }

    #[tokio::test]
    async fn test_send_email_invalid_address() {
        // Invalid `from_email` and `to` should trigger parse errors
        let sender = SmtpEmailSender::new("smtp.invalid.local", "user", "pass", "bad-from-email");

        let result = sender
            .send_email("not-an-email", "Subject", "<p>Body</p>")
            .await;

        assert!(
            result.is_err(),
            "Expected error due to invalid email address"
        );

        if let Err(e) = result {
            assert!(
                e.contains("Invalid"),
                "Expected 'Invalid' in error message, got: {}",
                e
            );
        }
    }

    #[tokio::test]
    async fn test_send_email_connection_failure() {
        // Force connection error by giving unreachable SMTP server
        let sender = SmtpEmailSender::new(
            "smtp.this-will-fail.local",
            "user",
            "pass",
            "from@example.com",
        );

        // This will panic at `.unwrap()` in `relay()` if domain is invalid DNS
        // So we make sure to use a *syntactically valid* domain that doesn't resolve
        let result = sender
            .send_email("to@example.com", "Subject", "<p>Body</p>")
            .await;

        assert!(
            result.is_err(),
            "Expected connection failure to produce error"
        );
    }
    #[tokio::test]
    async fn test_send_email_body_build_failure() {
        struct DummyMailer;
        #[async_trait]
        impl Mailer for DummyMailer {
            async fn send(&self, _: Message) -> Result<(), String> {
                panic!("Should not reach send()");
            }
        }

        let sender = SmtpEmailSender::new_with_mailer(Box::new(DummyMailer), "");

        // "" is a valid parseable address (parsed as empty local part), but invalid for email sending
        let result = sender
            .send_email("receiver@xample.com", "", "<p>Body</p>")
            .await;

        match result {
            Err(e) => {
                println!("Got error: {}", e); // Add this
                assert!(
                    e.contains("InvalidInput"),
                    "Unexpected error message: {}",
                    e
                );
            }
            _ => panic!("Expected an error"),
        }
    }
}
