use crate::email::application::ports::outgoing::email_sender::EmailSender;
use async_trait::async_trait;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

pub struct SmtpEmailSender {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from_email: String,
}

impl SmtpEmailSender {
    pub fn new(
        smtp_server: &str,
        smtp_username: &str,
        smtp_password: &str,
        from_email: &str,
    ) -> Self {
        let creds = Credentials::new(smtp_username.to_string(), smtp_password.to_string());
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(smtp_server)
            .unwrap()
            .credentials(creds)
            .build();

        Self {
            mailer,
            from_email: from_email.to_string(),
        }
    }
}

#[async_trait]
impl EmailSender for SmtpEmailSender {
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
        let email = Message::builder()
            .from(self.from_email.parse().unwrap())
            .to(to.parse().unwrap())
            .subject(subject)
            .body(body.to_string())
            .map_err(|e| e.to_string())?;

        self.mailer.send(email).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}
