/// Domain types: the vocabulary the business rules are written in.
pub mod domain;
/// Small shared collaborators that are not themselves use cases.
pub mod helpers;
/// Coordinators that compose several use cases into one operation.
pub mod orchestrator;
pub mod ports;
/// Implementations of this module's use-case contracts.
pub mod services;
pub mod use_cases;
