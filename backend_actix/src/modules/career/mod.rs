//! Job postings, applications, and what happened to them.
//!
//! The tracker is one row per application: what you applied to, with which CV,
//! where it got to, and what you owe it next. `cv_snapshots` lives in the `cv`
//! module — freezing a CV is a CV concern — and is reached from here through
//! the [`CvSnapshotter`](crate::career::application::ports::outgoing::CvSnapshotter) port.

/// Web routes and the Postgres stores.
pub mod adapter;
/// Ports, services and the use-case bundle.
pub mod application;
/// Jobs, applications, and the status an application has reached.
pub mod domain;
