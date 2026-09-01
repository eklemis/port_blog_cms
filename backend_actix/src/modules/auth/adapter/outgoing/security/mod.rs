/// The Argon2 `PasswordHasher` used in production.
pub mod argon2_hasher;
/// A bcrypt `PasswordHasher`, kept for hashes written before the Argon2 switch.
pub mod bcrypt_hasher;
