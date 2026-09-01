//! Auth's ports: what the module offers, and what it needs.
//!
//! `incoming` is what this module offers its adapters; `outgoing` is what it
//! asks of them. Both are traits, and neither knows who implements it. See
//! `docs/ARCHITECTURE.md`.

/// Driven side: HTTP route handlers, DTOs and error mapping.
pub mod incoming;
/// Driving side: the concrete implementations of this module's outgoing ports.
pub mod outgoing;
