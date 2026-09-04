//! The AI surfaces' application layer.

/// The grouped handles the route layer resolves through `AppState`.
pub mod ai_use_cases;
/// The module's boundaries.
pub mod ports;
/// Implementations of this module's use-case contracts.
pub mod service;
