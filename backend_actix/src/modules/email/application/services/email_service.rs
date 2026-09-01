use crate::email::application::ports::outgoing::email_sender::EmailSender;
use crate::email::application::ports::outgoing::password_reset_notifier::PasswordResetNotifier;
use crate::email::application::ports::outgoing::user_email_notifier::{
    UserEmailNotificationError, UserEmailNotifier,
};
use crate::email::application::ports::outgoing::Recipient;

/// Composes and sends the two notification emails.
///
/// Generic only over the transport. It does not mint tokens — callers pass one
/// in — which is what keeps `email` free of any dependency on `auth`.
#[derive(Clone, Debug)]
pub struct UserEmailService<E>
where
    E: EmailSender + Send + Sync,
{
    email_sender: E,
    app_url: String,
    /// Frontend route that accepts a reset token. Kept separate from `app_url`
    /// so the two links can move independently.
    reset_url: String,
}

impl<E> UserEmailService<E>
where
    E: EmailSender + Send + Sync,
{
    /// Builds the service.
    ///
    /// `app_url` is the frontend route that accepts a verification token and
    /// `reset_url` the one that accepts a reset token; each has the token
    /// appended as a path segment.
    pub fn new(email_sender: E, app_url: String, reset_url: String) -> Self {
        Self {
            email_sender,
            app_url,
            reset_url,
        }
    }

    fn create_password_reset_email(&self, username: &str, reset_token: &str) -> (String, String) {
        let reset_link = format!("{}/{}", self.reset_url, reset_token);

        let subject = "Reset Your Password".to_string();
        let html_body = format!(
            r#"
            <p>Hi {username},</p>
            <p>We received a request to reset your Ekstion password.</p>
            <p><a href="{reset_link}">Choose a new password</a></p>
            <p>This link expires shortly. If you did not ask for a reset you can
               ignore this email &mdash; your password will not change.</p>
            "#
        );

        (subject, html_body)
    }

    fn create_verification_email(
        &self,
        username: &str,
        verification_token: &str,
    ) -> (String, String) {
        let verification_link = format!("{}/{}", self.app_url, verification_token);

        let subject = "Verify Your Email".to_string();
        let html_body = format!(
            r#"
            <p>Hi {},</p>
            <p>Welcome to Ekstion! We're excited to have you on board.</p>
            <p>To complete your registration, click the button below:</p>
            <p>
                <a href="{}" style="display: inline-block; padding: 10px 20px; background-color: #007BFF; color: white; text-decoration: none; border-radius: 5px;">
                    Verify Your Email
                </a>
            </p>
            <p><strong>Note:</strong> This link is valid for 24 hours.</p>
            <p>Thanks,<br>The Ekstion Team</p>
            "#,
            username, verification_link
        );

        (subject, html_body)
    }
}

#[async_trait::async_trait]
impl<E> UserEmailNotifier for UserEmailService<E>
where
    E: EmailSender + Send + Sync,
{
    async fn send_verification_email(
        &self,
        recipient: &Recipient,
        verification_token: &str,
    ) -> Result<(), UserEmailNotificationError> {
        let (subject, body) =
            self.create_verification_email(&recipient.username, verification_token);

        self.email_sender
            .send_email(&recipient.email, &subject, &body)
            .await
            .map_err(UserEmailNotificationError::EmailSendingFailed)?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl<E> PasswordResetNotifier for UserEmailService<E>
where
    E: EmailSender + Send + Sync,
{
    async fn send_password_reset_email(
        &self,
        recipient: &Recipient,
        reset_token: &str,
    ) -> Result<(), UserEmailNotificationError> {
        let (subject, body) = self.create_password_reset_email(&recipient.username, reset_token);

        self.email_sender
            .send_email(&recipient.email, &subject, &body)
            .await
            .map_err(|e| UserEmailNotificationError::EmailSendingFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct SpySender {
        sent: Mutex<Vec<(String, String, String)>>,
        fail: bool,
    }

    #[async_trait]
    impl EmailSender for SpySender {
        async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
            self.sent
                .lock()
                .unwrap()
                .push((to.to_string(), subject.to_string(), body.to_string()));
            if self.fail {
                return Err("smtp refused".to_string());
            }
            Ok(())
        }
    }

    fn service(sender: SpySender) -> UserEmailService<SpySender> {
        UserEmailService::new(
            sender,
            "https://app.example.com/verify".to_string(),
            "https://app.example.com/reset".to_string(),
        )
    }

    fn a_recipient() -> Recipient {
        Recipient::new("john@example.com", "john")
    }

    // ------------------------------------------------------------------
    // Verification email
    // ------------------------------------------------------------------

    /// The link must be app_url + token and nothing else. 7658849 changed this
    /// from an API path to a frontend handler path, so the shape is load-bearing
    /// and worth pinning.
    #[tokio::test]
    async fn verification_email_links_to_the_configured_handler() {
        let svc = service(SpySender::default());

        svc.send_verification_email(&a_recipient(), "tok-123")
            .await
            .unwrap();

        let sent = svc.email_sender.sent.lock().unwrap();
        let (to, subject, body) = &sent[0];
        assert_eq!(to, "john@example.com");
        assert_eq!(subject, "Verify Your Email");
        assert!(
            body.contains("https://app.example.com/verify/tok-123"),
            "body did not carry the expected link: {body}"
        );
        assert!(body.contains("john"), "greeting should name the user");
    }

    #[tokio::test]
    async fn verification_email_reports_a_delivery_failure() {
        let svc = service(SpySender {
            fail: true,
            ..Default::default()
        });

        let err = svc
            .send_verification_email(&a_recipient(), "tok-123")
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            UserEmailNotificationError::EmailSendingFailed(m) if m.contains("smtp refused")
        ));
    }

    // ------------------------------------------------------------------
    // Password reset email
    // ------------------------------------------------------------------

    /// reset_url is a separate field from app_url so the two links can move
    /// independently; this asserts the reset mail uses the reset one.
    #[tokio::test]
    async fn reset_email_uses_the_reset_url_not_the_verification_url() {
        let svc = service(SpySender::default());

        svc.send_password_reset_email(&Recipient::new("jane@example.com", "jane"), "reset-tok")
            .await
            .unwrap();

        let sent = svc.email_sender.sent.lock().unwrap();
        let (to, subject, body) = &sent[0];
        assert_eq!(to, "jane@example.com");
        assert_eq!(subject, "Reset Your Password");
        assert!(
            body.contains("https://app.example.com/reset/reset-tok"),
            "body did not carry the reset link: {body}"
        );
        assert!(
            !body.contains("/verify/"),
            "reset mail must not point at the verification handler: {body}"
        );
    }

    /// The copy tells a recipient who did not request it that ignoring the mail
    /// is safe. That matters because the endpoint mails anyone whose address is
    /// entered, so uninvolved people receive it.
    #[tokio::test]
    async fn reset_email_tells_an_unintended_recipient_to_ignore_it() {
        let svc = service(SpySender::default());

        svc.send_password_reset_email(&Recipient::new("jane@example.com", "jane"), "reset-tok")
            .await
            .unwrap();

        let sent = svc.email_sender.sent.lock().unwrap();
        assert!(sent[0].2.contains("ignore this email"));
    }

    #[tokio::test]
    async fn reset_email_reports_a_delivery_failure() {
        let svc = service(SpySender {
            fail: true,
            ..Default::default()
        });

        let err = svc
            .send_password_reset_email(&Recipient::new("jane@example.com", "jane"), "t")
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            UserEmailNotificationError::EmailSendingFailed(m) if m.contains("smtp refused")
        ));
    }
}
