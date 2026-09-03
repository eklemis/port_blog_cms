//! What cv needs from the outside: a CV store split into write, read and lifecycle ports.

mod cv_repository;
pub use cv_repository::{CVRepository, CVRepositoryError, CreateCVData, PatchCVData, UpdateCVData};
// Optionally re-export if you want direct referencing:
// pub use cv_repository::CVRepository;
mod cv_query;
pub use cv_query::*;

mod cv_archiver;
pub use cv_archiver::{CVArchiver, CVArchiverError};

mod cv_snapshot_store;
pub use cv_snapshot_store::{CvSnapshot, CvSnapshotStore, CvSnapshotStoreError};
