//! What career needs from the outside: a store for jobs, one for
//! applications, and a way to freeze a CV without knowing how CVs work.

mod application_store;
mod cv_snapshotter;
mod job_store;

pub use application_store::{
    ApplicationStore, ApplicationStoreError, CreateApplicationData, PatchApplicationData,
};
pub use cv_snapshotter::{CvSnapshotter, CvSnapshotterError};
pub use job_store::{CreateJobData, JobStore, JobStoreError, PatchJobData};
