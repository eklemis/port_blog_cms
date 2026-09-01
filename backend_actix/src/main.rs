//! Binary entry point.
//!
//! Deliberately thin: everything the server does lives in the library so it
//! can be documented by rustdoc and reached from tests. This file only does
//! the one thing a library must not do at import time — install a process-wide
//! crypto provider — and then hands off to [`backend_actix::start`].

fn main() {
    // Must run before any TLS connection is opened. rustls refuses to pick a
    // provider implicitly when more than one is available, and both the Redis
    // and Postgres clients open TLS connections during `start`.
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    if let Err(e) = backend_actix::start() {
        eprintln!("Error starting app: {e}");
        std::process::exit(1);
    }
}
