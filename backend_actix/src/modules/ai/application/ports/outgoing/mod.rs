//! What ai needs from the outside: somewhere to count generations, and a
//! model to ask.

mod text_generator;
mod usage_counter;

pub use text_generator::{Generation, GenerationError, GenerationRequest, TextGenerator, Usage};
pub use usage_counter::{UsageCounter, UsageCounterError};
