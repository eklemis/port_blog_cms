use crate::auth::application::ports::incoming::password_policy::{
    PasswordPolicy, PasswordPolicyError,
};

pub struct BasicPasswordPolicy;

impl PasswordPolicy for BasicPasswordPolicy {
    fn validate(&self, password: &str) -> Result<(), PasswordPolicyError> {
        if password.len() < 12 {
            return Err(PasswordPolicyError::TooShort);
        }

        if password.len() > 128 {
            return Err(PasswordPolicyError::TooLong);
        }

        Ok(())
    }
}
