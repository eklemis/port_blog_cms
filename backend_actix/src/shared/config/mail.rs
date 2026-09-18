//! Reading the mail configuration, and refusing the values that look fine and
//! are not.
//!
//! Every function here is pure and takes the environment as arguments, so the
//! rules are tested rather than exercised by starting a server.
//!
//! Three faults this exists to prevent, all three of which happened:
//!
//! 1. `PASSWORD_RESET_HANDLER_URL` was unset, so the link fell back to a
//!    hardcoded `0.0.0.0:5173/password-reset`. No scheme, so a browser reads it
//!    as a relative path, and `0.0.0.0` is not an address anyone can browse.
//!    The email arrived and the link did nothing.
//! 2. The test branch read `SMTP_HOST` while the configuration only set
//!    `SMTP_SERVER`, so the host silently defaulted to `localhost` and the
//!    configured value was ignored.
//! 3. Nothing was listening on the mail port, so every send failed with
//!    connection-refused — visible only in a log line, because the endpoint
//!    answers "if that email is registered, a link has been sent" either way.
//! 4. The correction to the first fault kept its path. `/password-reset` is not
//!    a route the frontend has — it serves `auth/reset/[token]` — so the link
//!    became well-formed and still went nowhere. That failure is worse than a
//!    malformed one: mail sends, the link opens, the app answers 404, and every
//!    hop but the last looks right, so the frontend gets blamed for it.
//!
//! Hence the rule these functions enforce: **a link target is configuration,
//! never a default.** It is a frontend route, the backend cannot check that it
//! exists, and a guess that is wrong produces a product that is broken quietly.
//! Missing is fatal in every environment, like `DATABASE_URL` — loud, at
//! startup, in front of whoever deployed. `.env.example` carries the
//! development values.

use std::time::Duration;

/// Where a link in an email should point.
///
/// Held as a string because it is concatenated with a token and handed to a
/// mail template, never parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerUrl(String);

impl HandlerUrl {
    /// The configured value, ready to have `/{token}` appended.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for HandlerUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What went wrong with a configured link target.
#[derive(Debug, PartialEq, Eq)]
pub enum HandlerUrlError {
    /// Not set. There is no default, in any environment.
    Missing {
        /// The variable that was expected.
        var: String,
    },
    /// Set to something a browser cannot follow.
    Unusable {
        /// The variable it was read from.
        var: String,
        /// What it was set to, quoted back so it can be found and corrected.
        value: String,
        /// Why a browser could not follow it.
        why: String,
    },
}

impl std::fmt::Display for HandlerUrlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing { var } => write!(
                f,
                "{var} is not set. It is a frontend route, which this service \
                 cannot verify and must not guess: a wrong one sends mail whose \
                 link opens a 404. Set it to the frontend origin plus the route, \
                 e.g. https://example.com/auth/reset — backend_actix/.env.example \
                 lists the development values."
            ),
            Self::Unusable { var, value, why } => write!(
                f,
                "{var} is set to {value:?}, which a browser cannot follow: {why}. \
                 Include the scheme, e.g. http://localhost:5173/auth/reset"
            ),
        }
    }
}

/// The link target for one kind of email, from configuration only.
///
/// There is no default, deliberately, and none for development either. The
/// value is a route belonging to another application: this service cannot ask
/// whether it exists, so any value it invents is a guess that will be right
/// until the day the frontend moves a route, and wrong silently after it.
///
/// A missing value stops the server, which is the cheap failure — it happens
/// once, at startup, in front of the person deploying. The alternative failure
/// happens to users, one forgotten password at a time, and reads as the
/// frontend's fault.
pub fn handler_url(var: &str, configured: Option<String>) -> Result<HandlerUrl, HandlerUrlError> {
    match configured.as_deref().map(str::trim) {
        Some("") | None => Err(HandlerUrlError::Missing {
            var: var.to_string(),
        }),
        Some(value) => match unusable_reason(value) {
            Some(why) => Err(HandlerUrlError::Unusable {
                var: var.to_string(),
                value: value.to_string(),
                why,
            }),
            None => Ok(HandlerUrl(value.trim_end_matches('/').to_string())),
        },
    }
}

/// Why a browser could not follow this, or `None` if it could.
fn unusable_reason(value: &str) -> Option<String> {
    let Some(rest) = value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"))
    else {
        return Some("it has no http:// or https:// scheme".to_string());
    };

    let host = rest.split(['/', ':']).next().unwrap_or("");

    if host.is_empty() {
        return Some("it names no host".to_string());
    }

    // 0.0.0.0 is the address a server binds to in order to accept connections
    // on every interface. It is not an address a client can connect *to*, so a
    // link containing it is broken wherever it is opened. It appears here
    // because it is the natural thing to copy out of a HOST variable.
    if host == "0.0.0.0" {
        return Some(
            "0.0.0.0 is a bind address, not one a browser can open — use localhost \
             for development or the real hostname in production"
                .to_string(),
        );
    }

    None
}

/// Which host to send mail through in development, and what to warn about.
///
/// Only `SMTP_HOST` is read, deliberately: `SMTP_SERVER` is the production
/// relay, and quietly borrowing it here would point development mail at the
/// real relay — the one outcome worse than mail that does not send. But a
/// configuration that sets only `SMTP_SERVER` is a configuration whose author
/// expected it to be used, so that combination is named rather than ignored.
pub fn development_smtp_host(
    smtp_host: Option<String>,
    smtp_server: Option<String>,
) -> (String, Option<String>) {
    match smtp_host.as_deref().map(str::trim) {
        Some(host) if !host.is_empty() => (host.to_string(), None),
        _ => {
            let warning = smtp_server
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|server| {
                    format!(
                        "SMTP_HOST is not set, so development mail goes to localhost. \
                     SMTP_SERVER is set to {server:?} and is deliberately NOT used here: \
                     it is the production relay, and development must not send real mail. \
                     Set SMTP_HOST if you meant something other than localhost."
                    )
                });
            ("localhost".to_string(), warning)
        }
    }
}

/// Whether anything is accepting mail at `host:port`.
///
/// Run at startup so a mail server that is not there is reported when the
/// process starts, rather than one user at a time when a reset silently fails.
/// The endpoint cannot report it: answering "sent" whether or not the address
/// exists is what stops it being used to discover accounts, so a failed send
/// has to be visible here instead.
pub async fn smtp_reachable(host: &str, port: u16, timeout: Duration) -> Result<(), String> {
    let address = format!("{host}:{port}");

    match tokio::time::timeout(timeout, tokio::net::TcpStream::connect(&address)).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(format!("{address} refused the connection: {e}")),
        Err(_) => Err(format!(
            "{address} did not answer within {}s",
            timeout.as_secs()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_configured_url_is_taken_as_given() {
        let url = handler_url(
            "PASSWORD_RESET_HANDLER_URL",
            Some("https://example.com/auth/reset".to_string()),
        )
        .expect("a well-formed url is accepted");

        assert_eq!(url.as_str(), "https://example.com/auth/reset");
    }

    /// The trailing slash would otherwise produce `…/auth/reset//token`.
    #[test]
    fn a_trailing_slash_is_dropped() {
        let url = handler_url(
            "PASSWORD_RESET_HANDLER_URL",
            Some("https://example.com/auth/reset/".to_string()),
        )
        .expect("accepted");

        assert_eq!(url.as_str(), "https://example.com/auth/reset");
    }

    /// The first failure that shipped: no scheme, so a browser reads the value
    /// as a relative path and the link goes nowhere.
    #[test]
    fn a_url_without_a_scheme_is_refused() {
        let err = handler_url(
            "PASSWORD_RESET_HANDLER_URL",
            Some("0.0.0.0:5173/password-reset".to_string()),
        )
        .expect_err("must not be accepted");

        assert!(
            matches!(&err, HandlerUrlError::Unusable { why, .. } if why.contains("scheme")),
            "got {err:?}"
        );
    }

    /// A bind address is not a destination, however well-formed the URL is.
    #[test]
    fn a_bind_address_is_refused_even_with_a_scheme() {
        let err = handler_url(
            "VERIFICATION_HANDLER_URL",
            Some("http://0.0.0.0:5173/email/verification".to_string()),
        )
        .expect_err("0.0.0.0 cannot be opened by a browser");

        assert!(
            matches!(&err, HandlerUrlError::Unusable { why, .. } if why.contains("bind address")),
            "got {err:?}"
        );
    }

    /// No environment gets a default, development included.
    ///
    /// A development default is where the last two faults came from: the first
    /// guess was unusable, the correction guessed a route the frontend does not
    /// serve, and both shipped because a guess had somewhere to live. The
    /// server cannot check a route in another application, so it does not try.
    #[test]
    fn missing_is_fatal_in_every_environment() {
        let err = handler_url("PASSWORD_RESET_HANDLER_URL", None)
            .expect_err("a link target is configuration, not a guess");

        assert!(
            matches!(err, HandlerUrlError::Missing { .. }),
            "got {err:?}"
        );

        let message = err.to_string();
        assert!(
            message.contains(".env.example"),
            "the message must say where the values are: {message}"
        );
    }

    /// An empty value is a value someone tried to set, and it is unusable.
    #[test]
    fn an_empty_value_is_treated_as_unset() {
        let err = handler_url("PASSWORD_RESET_HANDLER_URL", Some("   ".to_string()))
            .expect_err("blank is not configured");

        assert!(
            matches!(err, HandlerUrlError::Missing { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn smtp_host_is_used_when_set() {
        let (host, warning) = development_smtp_host(
            Some("mail.internal".to_string()),
            Some("smtp-relay.example.com".to_string()),
        );

        assert_eq!(host, "mail.internal");
        assert_eq!(warning, None);
    }

    /// The mismatch that made a configured value look effective and not be.
    #[test]
    fn smtp_server_alone_is_reported_and_not_borrowed() {
        let (host, warning) =
            development_smtp_host(None, Some("smtp-relay.example.com".to_string()));

        assert_eq!(
            host, "localhost",
            "development must not send through the production relay"
        );
        let warning = warning.expect("the mismatch must be named");
        assert!(warning.contains("SMTP_HOST"), "got {warning}");
        assert!(
            warning.contains("smtp-relay.example.com"),
            "the ignored value must appear, or nobody will find it: {warning}"
        );
    }

    #[test]
    fn neither_set_is_localhost_and_says_nothing() {
        let (host, warning) = development_smtp_host(None, None);

        assert_eq!(host, "localhost");
        assert_eq!(warning, None, "there is no mismatch to report");
    }

    #[tokio::test]
    async fn an_unreachable_mail_server_is_an_error_rather_than_a_hang() {
        // Port 1 on localhost: nothing listens there, and it fails fast.
        let result = smtp_reachable("127.0.0.1", 1, Duration::from_secs(2)).await;

        let message = result.expect_err("nothing accepts mail on port 1");
        assert!(message.contains("127.0.0.1:1"), "got {message}");
    }
}
