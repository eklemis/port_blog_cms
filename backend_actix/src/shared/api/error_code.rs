//! The error-code vocabulary every endpoint speaks.
//!
//! Every error response carries `{ success: false, error: { code, message } }`.
//! The `message` is prose and may change; **the `code` is a contract** that
//! clients branch on, so it must not.
//!
//! Codes used to be bare `&str` literals passed into [`ApiResponse::error`],
//! which meant a typo minted a new code, a rename broke clients silently, and
//! nothing could enumerate the vocabulary. They are an enum instead, so the
//! compiler rejects unknown codes and `docs/API_ERRORS.md` plus the OpenAPI
//! schema are generated from one list.
//!
//! [`ApiResponse::error`]: crate::shared::api::ApiResponse::error
//!
//! # HTTP status is not part of a code
//!
//! Three codes are deliberately emitted with two different statuses, because
//! the same condition means different things at different endpoints:
//!
//! - `TOKEN_INVALID` / `TOKEN_EXPIRED` — 400 from email verification, where the
//!   token is a malformed *input* in the URL; 401 from token refresh, where it
//!   is a *credential*.
//! - `INVALID_TOKEN_TYPE` — 401 from the auth extractor, where an access token
//!   was expected; 400 from the refresh endpoint, where a refresh token was.
//!
//! The status therefore stays at the call site. [`ErrorCode::typical_status`]
//! reports the status a code is usually paired with, for documentation only —
//! it is not used to build responses.

use actix_web::http::StatusCode;

/// Declares the vocabulary once and derives everything from it.
///
/// Adding a variant here is all that is needed: `as_str`, `ALL`, `description`
/// and the generated documentation follow automatically, and the OpenAPI
/// schema picks it up through `ALL`.
macro_rules! error_codes {
    ($( $group:literal | $variant:ident => $wire:literal, $status:ident, $doc:literal ; )*) => {
        /// A machine-readable error code returned in `error.code`.
        ///
        /// See the module documentation for why the HTTP status is not encoded
        /// here, and `docs/API_ERRORS.md` for the rendered reference.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum ErrorCode {
            $( #[doc = $doc] $variant, )*
        }

        impl ErrorCode {
            /// Every code, in declaration order. Used to generate the
            /// documentation and the OpenAPI enum.
            pub const ALL: &'static [ErrorCode] = &[ $( ErrorCode::$variant ),* ];

            /// The exact string that appears in `error.code`.
            pub const fn as_str(&self) -> &'static str {
                match self { $( ErrorCode::$variant => $wire, )* }
            }

            /// What the code means, for humans reading the generated reference.
            pub const fn description(&self) -> &'static str {
                match self { $( ErrorCode::$variant => $doc, )* }
            }

            /// The section this code belongs to in the generated reference.
            pub const fn group(&self) -> &'static str {
                match self { $( ErrorCode::$variant => $group, )* }
            }

            /// The status this code is usually paired with.
            ///
            /// Documentation only — responses take their status from the call
            /// site, and three codes appear with two statuses. See the module
            /// documentation.
            pub fn typical_status(&self) -> StatusCode {
                match self { $( ErrorCode::$variant => StatusCode::$status, )* }
            }
        }
    };
}

error_codes! {
    // ── Authentication and tokens ────────────────────────────────────────
    "Authentication and tokens" | MissingAuthHeader   => "MISSING_AUTH_HEADER", UNAUTHORIZED,
        "No Authorization header, or it was not a well-formed Bearer token.";
    "Authentication and tokens" | InvalidCredentials  => "INVALID_CREDENTIALS", UNAUTHORIZED,
        "Email and password did not match an account. Deliberately does not say which was wrong.";
    "Authentication and tokens" | InvalidToken        => "INVALID_TOKEN", UNAUTHORIZED,
        "The token could not be verified, or has expired.";
    "Authentication and tokens" | TokenInvalid        => "TOKEN_INVALID", UNAUTHORIZED,
        "The token's signature or structure is not valid. 401 when it was offered as a credential, 400 when it arrived as a URL segment.";
    "Authentication and tokens" | TokenExpired        => "TOKEN_EXPIRED", UNAUTHORIZED,
        "The token's expiry has passed. 401 on refresh, 400 on email verification.";
    "Authentication and tokens" | TokenNotYetValid    => "TOKEN_NOT_YET_VALID", BAD_REQUEST,
        "The token's not-before claim is in the future. Usually a clock-skew problem.";
    "Authentication and tokens" | InvalidTokenType    => "INVALID_TOKEN_TYPE", UNAUTHORIZED,
        "A token of the wrong kind was supplied — an access token where a refresh token was required, or the reverse.";
    "Authentication and tokens" | InvalidResetToken   => "INVALID_RESET_TOKEN", UNAUTHORIZED,
        "The password-reset token is invalid or has expired.";
    "Authentication and tokens" | EmailNotVerified    => "EMAIL_NOT_VERIFIED", FORBIDDEN,
        "The account exists but its email address has not been confirmed.";
    "Authentication and tokens" | UserDeleted         => "USER_DELETED", FORBIDDEN,
        "The account has been soft-deleted and cannot be used.";

    // ── Authorisation ────────────────────────────────────────────────────
    "Authorisation" | Forbidden           => "FORBIDDEN", FORBIDDEN,
        "The caller is authenticated but does not own the resource.";
    "Authorisation" | UserUnauthorized    => "USER_UNAUTHORIZED", UNAUTHORIZED,
        "The caller may not perform this action on this account.";
    "Authorisation" | CvUnauthorized      => "CV_UNAUTHORIZED", FORBIDDEN,
        "The caller does not own this CV.";
    "Authorisation" | PostUnauthorized    => "POST_UNAUTHORIZED", FORBIDDEN,
        "The caller does not own this blog post.";

    // ── Not found ────────────────────────────────────────────────────────
    "Not found" | UserNotFound        => "USER_NOT_FOUND", NOT_FOUND,
        "No user matches the given id or username.";
    "Not found" | CvNotFound          => "CV_NOT_FOUND", NOT_FOUND,
        "No CV matches the given id, or it is not visible to the caller.";
    "Not found" | PostNotFound        => "POST_NOT_FOUND", NOT_FOUND,
        "No blog post matches the given id or slug.";
    "Not found" | ProjectNotFound     => "PROJECT_NOT_FOUND", NOT_FOUND,
        "No project matches the given id or slug.";
    "Not found" | TopicNotFound       => "TOPIC_NOT_FOUND", NOT_FOUND,
        "No topic matches the given id.";
    "Not found" | MediaNotFound       => "MEDIA_NOT_FOUND", NOT_FOUND,
        "No media item matches the given id.";
    "Not found" | VariantNotFound     => "VARIANT_NOT_FOUND", NOT_FOUND,
        "The media item exists but not in the requested size.";
    "Not found" | TargetNotFound      => "TARGET_NOT_FOUND", BAD_REQUEST,
        "The attachment target named in the request does not exist.";

    // ── Conflict ─────────────────────────────────────────────────────────
    "Conflict" | UserAlreadyExists   => "USER_ALREADY_EXISTS", CONFLICT,
        "That email or username is already registered.";
    "Conflict" | TopicAlreadyExists  => "TOPIC_ALREADY_EXISTS", CONFLICT,
        "The caller already owns a topic with that title.";
    "Conflict" | SlugAlreadyExists   => "SLUG_ALREADY_EXISTS", CONFLICT,
        "Another post or project already uses that slug.";

    // ── Request validation ───────────────────────────────────────────────
    "Request validation" | ValidationError     => "VALIDATION_ERROR", BAD_REQUEST,
        "The request body could not be deserialised. The message carries the parser's detail.";
    "Request validation" | InvalidRequest      => "INVALID_REQUEST", BAD_REQUEST,
        "The request is structurally valid but not a combination the endpoint accepts.";
    "Request validation" | MissingField        => "MISSING_FIELD", BAD_REQUEST,
        "A required field was absent.";
    "Request validation" | InvalidEmail        => "INVALID_EMAIL", BAD_REQUEST,
        "The email address is not well formed.";
    "Request validation" | InvalidPassword     => "INVALID_PASSWORD", BAD_REQUEST,
        "The password does not meet the strength policy.";
    "Request validation" | InvalidUsername     => "INVALID_USERNAME", BAD_REQUEST,
        "The username contains disallowed characters or is the wrong length.";
    "Request validation" | InvalidFullName     => "INVALID_FULL_NAME", BAD_REQUEST,
        "The full name is empty or too long.";
    "Request validation" | InvalidSlug         => "INVALID_SLUG", BAD_REQUEST,
        "The slug is empty, too long, or contains characters outside [a-z0-9-].";
    "Request validation" | InvalidTitle        => "INVALID_TITLE", BAD_REQUEST,
        "The title is not acceptable for this resource.";
    "Request validation" | EmptyTitle          => "EMPTY_TITLE", BAD_REQUEST,
        "The title was empty once trimmed.";
    "Request validation" | TitleTooLong        => "TITLE_TOO_LONG", BAD_REQUEST,
        "The title exceeds the maximum length.";
    "Request validation" | InvalidContent      => "INVALID_CONTENT", BAD_REQUEST,
        "The post body is empty or otherwise unacceptable.";

    // ── Uploads ──────────────────────────────────────────────────────────
    "Uploads" | InvalidFileName     => "INVALID_FILE_NAME", BAD_REQUEST,
        "The file name is empty, too long, or contains path separators.";
    "Uploads" | InvalidExtension    => "INVALID_EXTENSION", BAD_REQUEST,
        "The file extension is not one the upload policy allows.";
    "Uploads" | InvalidMimeType     => "INVALID_MIME_TYPE", BAD_REQUEST,
        "The declared MIME type is not one the upload policy allows.";
    "Uploads" | MimeExtensionMismatch => "MIME_EXTENSION_MISMATCH", BAD_REQUEST,
        "The declared MIME type and the file extension disagree.";
    "Uploads" | FileTooLarge        => "FILE_TOO_LARGE", BAD_REQUEST,
        "The declared file size exceeds the upload limit.";
    "Uploads" | InvalidDimensions   => "INVALID_DIMENSIONS", BAD_REQUEST,
        "The declared image dimensions are missing, zero, or above the limit.";

    // ── Media processing state ───────────────────────────────────────────
    "Media processing state" | MediaPending        => "MEDIA_PENDING", CONFLICT,
        "The upload has been registered but the file has not arrived yet.";
    "Media processing state" | MediaProcessing     => "MEDIA_PROCESSING", CONFLICT,
        "The file arrived but its variants are still being generated. Retry shortly.";
    "Media processing state" | MediaFailed         => "MEDIA_FAILED", CONFLICT,
        "Variant generation failed for this media item; it will not become available.";

    // ── Throttling and server faults ─────────────────────────────────────
    "Throttling and server faults" | StorageError        => "STORAGE_ERROR", BAD_GATEWAY,
        "The object store could not be reached or refused the operation. Upstream fault, not the caller's.";
    "Throttling and server faults" | RateLimited         => "RATE_LIMITED", TOO_MANY_REQUESTS,
        "The caller exceeded the endpoint's rate limit. The response carries Retry-After.";
    "Throttling and server faults" | InternalError       => "INTERNAL_ERROR", INTERNAL_SERVER_ERROR,
        "An unexpected server-side failure. The message is deliberately generic; details are logged, not returned.";
}

/// Publishes the vocabulary to the OpenAPI document as a string enum.
///
/// Built from [`ErrorCode::ALL`], so a new variant appears in the spec — and
/// in every generated client — without anyone remembering to update it.
impl utoipa::PartialSchema for ErrorCode {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        utoipa::openapi::ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::SchemaType::Type(
                utoipa::openapi::schema::Type::String,
            ))
            .description(Some(
                "Machine-readable error code. Stable contract: branch on this, \
                 not on `message`. See docs/API_ERRORS.md.",
            ))
            .enum_values(Some(ErrorCode::ALL.iter().map(|c| c.as_str())))
            .examples([serde_json::json!("USER_NOT_FOUND")])
            .into()
    }
}

impl utoipa::ToSchema for ErrorCode {
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("ErrorCode")
    }
}

impl ErrorCode {
    /// Renders `docs/API_ERRORS.md` from [`ErrorCode::ALL`].
    ///
    /// The committed file is checked against this in a test, so the reference
    /// cannot drift from the code. Regenerate with:
    ///
    /// ```bash
    /// UPDATE_DOCS=1 cargo test -p backend_actix api_errors_doc
    /// ```
    pub fn markdown_reference() -> String {
        let mut out = String::new();
        out.push_str(
"<!-- GENERATED FILE - DO NOT EDIT BY HAND.
     Source: src/shared/api/error_code.rs
     Regenerate: UPDATE_DOCS=1 cargo test -p backend_actix api_errors_doc -->

# API error codes

Every error response has the same shape:

```json
{
  \"success\": false,
  \"error\": { \"code\": \"USER_NOT_FOUND\", \"message\": \"User not found\" }
}
```

`code` is the contract. **Branch on `code`, never on `message`** — messages are
prose written for humans and change without notice.

## HTTP status is not implied by the code

The *typical status* column below is the status a code is usually returned
with, but it is not a guarantee: three codes are deliberately emitted with two
statuses, because the same condition means different things at different
endpoints.

| Code | Also returned as | Where, and why |
| --- | --- | --- |
| `TOKEN_INVALID` | 400 | Email verification, where the token is a malformed path segment rather than a credential. 401 on token refresh. |
| `TOKEN_EXPIRED` | 400 | Same reason as above. |
| `INVALID_TOKEN_TYPE` | 400 | The refresh endpoint, where a non-refresh token is bad input. 401 from the auth extractor, where an access token was required. |

Treat the status as the transport-level answer and the code as the specific
one.

");
        let mut group = "";
        for code in ErrorCode::ALL {
            if code.group() != group {
                group = code.group();
                out.push_str(&format!(
                    "## {group}\n\n| Code | Typical status | Meaning |\n| --- | --- | --- |\n"
                ));
            }
            let st = code.typical_status();
            out.push_str(&format!(
                "| `{}` | {} {} | {} |\n",
                code.as_str(),
                st.as_u16(),
                st.canonical_reason().unwrap_or(""),
                code.description()
            ));
            let last = ErrorCode::ALL.last().map(|c| c.as_str()) == Some(code.as_str());
            let next_group_differs = !last
                && ErrorCode::ALL
                    .iter()
                    .skip_while(|c| c.as_str() != code.as_str())
                    .nth(1)
                    .map(|c| c.group() != group)
                    .unwrap_or(false);
            if next_group_differs || last {
                out.push('\n');
            }
        }
        out.push_str(&format!(
            "---\n\n{} codes in total. This file is generated from \
             `src/shared/api/error_code.rs`; add a variant there and \
             regenerate.\n",
            ErrorCode::ALL.len()
        ));
        out
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl serde::Serialize for ErrorCode {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Two variants sharing a wire string would make the code ambiguous for
    /// clients and silently merge two conditions.
    #[test]
    fn wire_strings_are_unique() {
        let mut seen = HashSet::new();
        for code in ErrorCode::ALL {
            assert!(
                seen.insert(code.as_str()),
                "duplicate wire string: {}",
                code.as_str()
            );
        }
    }

    /// Clients pattern-match on these, and a lowercase or hyphenated code
    /// would break the convention every existing code follows.
    #[test]
    fn wire_strings_are_screaming_snake_case() {
        for code in ErrorCode::ALL {
            let s = code.as_str();
            assert!(!s.is_empty(), "empty wire string for {code:?}");
            assert!(
                s.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'),
                "not SCREAMING_SNAKE_CASE: {s}"
            );
            assert!(
                !s.starts_with('_') && !s.ends_with('_'),
                "stray underscore: {s}"
            );
        }
    }

    /// The generated reference is only useful if every entry says something.
    #[test]
    fn every_code_has_a_description() {
        for code in ErrorCode::ALL {
            let d = code.description();
            assert!(
                d.len() > 20,
                "description too thin for {}: {d:?}",
                code.as_str()
            );
            assert!(d.ends_with('.'), "description should be a sentence: {d:?}");
        }
    }

    /// `docs/API_ERRORS.md` is generated, not written. This fails if the
    /// committed file has drifted from the vocabulary — which happens whenever
    /// someone adds a code and forgets the doc.
    ///
    /// Regenerate with `UPDATE_DOCS=1 cargo test -p backend_actix api_errors_doc`.
    #[test]
    fn api_errors_doc_is_up_to_date() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/API_ERRORS.md");
        let generated = ErrorCode::markdown_reference();

        if std::env::var("UPDATE_DOCS").is_ok() {
            std::fs::write(path, &generated).expect("could not write docs/API_ERRORS.md");
            return;
        }

        let committed = std::fs::read_to_string(path).unwrap_or_default();
        assert_eq!(
            committed, generated,
            "docs/API_ERRORS.md is out of date. \
             Regenerate: UPDATE_DOCS=1 cargo test -p backend_actix api_errors_doc"
        );
    }

    #[test]
    fn display_and_serde_both_emit_the_wire_string() {
        assert_eq!(ErrorCode::UserNotFound.to_string(), "USER_NOT_FOUND");
        assert_eq!(
            serde_json::to_string(&ErrorCode::RateLimited).unwrap(),
            "\"RATE_LIMITED\""
        );
    }

    #[test]
    fn typical_status_is_a_client_or_server_error() {
        for code in ErrorCode::ALL {
            assert!(
                code.typical_status().is_client_error() || code.typical_status().is_server_error(),
                "{} maps to a non-error status",
                code.as_str()
            );
        }
    }
}
