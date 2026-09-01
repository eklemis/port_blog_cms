//! Email's ports. There are no incoming ports — nothing outside the process calls this module.
//!
//! `incoming` is what this module offers its adapters; `outgoing` is what it
//! asks of them. Both are traits, and neither knows who implements it. See
//! `docs/ARCHITECTURE.md`.

pub mod outgoing;
