//! Email's ports. There are no incoming ports — nothing outside the process calls this module.
//!
//! `incoming` is what this module offers its adapters; `outgoing` is what it
//! asks of them. Both are traits, and neither knows who implements it. See
//! `docs/ARCHITECTURE.md`.

/// Driving side: the concrete implementations of this module's outgoing ports.
pub mod outgoing;
