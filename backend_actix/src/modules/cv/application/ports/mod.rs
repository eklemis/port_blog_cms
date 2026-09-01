//! CV's ports.
//!
//! `incoming` is what this module offers its adapters; `outgoing` is what it
//! asks of them. Both are traits, and neither knows who implements it. See
//! `docs/ARCHITECTURE.md`.

// incoming ports are the use cases themselves
pub mod outgoing;
