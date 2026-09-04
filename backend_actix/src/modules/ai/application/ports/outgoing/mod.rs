//! What ai needs from the outside: somewhere to count generations.

mod usage_counter;

pub use usage_counter::{UsageCounter, UsageCounterError};
