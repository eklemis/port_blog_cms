//! The model-backed surfaces, and the allowance that governs them.
//!
//! Only the allowance exists so far. It is here first on purpose: a quota
//! retrofitted onto screens built assuming generation is free is the expensive
//! order to do this in, and counting from the start means the eventual limit
//! can be chosen from what people actually did.

/// Web routes and the Redis counter.
pub mod adapter;
/// Ports, services and the use-case bundle.
pub mod application;
/// Generation allowances and the period they run over.
pub mod domain;
