//! What auth needs from the outside: a user store, a token store, a token minter and a password hasher.

#![deny(missing_docs)]

pub mod token_repository;
pub mod user_query;
pub mod user_repository;

pub use user_query::UserQuery;
pub use user_repository::{UserRepository, UserRepositoryError};

pub mod password_hasher;
pub mod token_hasher;
pub mod token_provider;
