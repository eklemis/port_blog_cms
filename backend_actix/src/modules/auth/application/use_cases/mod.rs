#![deny(missing_docs)]

//! Auth's use cases.
//!
//! These predate the `ports/incoming` split that `blog`, `project`, `topic` and
//! `multimedia` use: each file here declares both the trait and the service
//! implementing it. See the convention section of `docs/ARCHITECTURE.md`; new
//! modules should follow the newer layout.

/// Registration: validate, hash the password, insert the user.
pub mod create_user;
/// Reads the authenticated user's own profile.
pub mod fetch_profile;
/// Exchanges credentials for an access and refresh token pair.
pub mod login_user;
/// Blacklists a refresh token so it stops being accepted.
pub mod logout_user;
/// Exchanges a refresh token for a fresh access token.
pub mod refresh_token;
/// Starts a password reset: mints a token and mails the link.
pub mod request_password_reset;
/// Completes a password reset by redeeming the token.
pub mod reset_password;
/// Soft-deletes the caller's own account.
pub mod soft_delete_user;
/// Edits the authenticated user's own profile.
pub mod update_profile;
/// Redeems an emailed verification token and marks the address confirmed.
pub mod verify_user_email;
