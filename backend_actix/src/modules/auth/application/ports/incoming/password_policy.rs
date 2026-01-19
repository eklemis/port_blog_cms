pub trait PasswordPolicy: Send + Sync {
    fn validate(&self, password: &str) -> Result<(), PasswordPolicyError>;
}

#[derive(Debug)]
pub enum PasswordPolicyError {
    TooShort,
    TooLong,
    TooWeak,
}
