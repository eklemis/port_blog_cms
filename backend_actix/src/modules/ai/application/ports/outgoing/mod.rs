//! What ai needs from the outside: somewhere to count generations, and a
//! model to ask.

mod drafting_context;
mod text_generator;
mod usage_counter;

pub use drafting_context::{
    DraftingContextError, DraftingContextReader, DraftingMaterial, PostingFetcher,
};
pub use text_generator::{
    Generation, GenerationError, GenerationEvent, GenerationRequest, GenerationStream,
    TextGenerator, Usage,
};
pub use usage_counter::{UsageCounter, UsageCounterError};
