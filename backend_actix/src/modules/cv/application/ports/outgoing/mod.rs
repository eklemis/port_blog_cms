pub mod cv_repository;
pub use cv_repository::{CVRepository, CVRepositoryError, CreateCVData, UpdateCVData};
// Optionally re-export if you want direct referencing:
// pub use cv_repository::CVRepository;
