use serde::{Deserialize, Serialize};

use crate::auth::application::domain::entities::UserId;

/// See the module documentation.
#[derive(Serialize, Deserialize, Debug)]
pub struct Topic {
    owner: UserId,
    title: String,
    description: String,
}
