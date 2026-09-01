//! What multimedia needs from the outside: object storage, and the database rows tracking each upload.

/// Google Cloud Storage adapters.
pub mod cloud_storage;
/// Database adapters for media rows.
pub mod db;
