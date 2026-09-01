//! Write-side port for CVs, plus the owner-facing fetches the write path needs.

// cv_repository.rs
use crate::cv::domain::entities::{
    CVInfo, ContactDetail, CoreSkill, Education, Experience, HighlightedProject,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Why a CV write failed.
#[derive(Debug, Clone)]
pub enum CVRepositoryError {
    /// No CV matched the id.
    NotFound,
    /// The store could not be reached.
    DatabaseError(String),
}

/// Creates, updates and fetches CVs.
///
/// Lifecycle transitions live in [`CVArchiver`](super::cv_archiver::CVArchiver);
/// listings and public reads in [`CVQuery`](super::cv_query::CVQuery).
#[async_trait]
pub trait CVRepository: Send + Sync {
    /// Every CV belonging to a user. An empty `Vec` when they have none —
    /// not an error.
    async fn fetch_cv_by_user_id(&self, user_id: Uuid) -> Result<Vec<CVInfo>, CVRepositoryError>;
    /// One CV by id, regardless of owner. `Ok(None)` when absent, so the
    /// caller must perform its own ownership check before acting.
    async fn fetch_cv_by_id(&self, cv_id: Uuid) -> Result<Option<CVInfo>, CVRepositoryError>;
    /// Inserts a CV.
    async fn create_cv(
        &self,
        user_id: Uuid,
        cv_data: CreateCVData,
    ) -> Result<CVInfo, CVRepositoryError>;
    /// Replaces a CV's contents.
    async fn update_cv(
        &self,
        cv_id: Uuid,
        cv_data: UpdateCVData,
    ) -> Result<CVInfo, CVRepositoryError>;
}

// Separate struct for creating CV (no ID needed from user)
/// Everything needed to insert a CV.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCVData {
    /// Job title shown under the display name.
    pub role: String,
    /// Free-form introduction.
    pub bio: String,
    /// Name shown on the CV, which need not match the account's full name.
    pub display_name: String,
    /// Portrait image. Empty when none has been set.
    pub photo_url: String,
    /// Headline skills, in display order.
    pub core_skills: Vec<CoreSkill>,
    /// Education entries, in display order.
    pub educations: Vec<Education>,
    /// Work history, in display order.
    pub experiences: Vec<Experience>,
    /// Projects featured on the CV, in display order.
    pub highlighted_projects: Vec<HighlightedProject>,
    /// Contact rows — email, links, phone. Public on a published CV.
    pub contact_info: Vec<ContactDetail>,
}

// Separate struct for updating CV
/// A full replacement carries the same fields as a creation, so the two share
/// one type. Contrast [`PatchCVData`], which is partial.
pub type UpdateCVData = CreateCVData;

/// A partial CV update. Omitted fields are left as stored.
#[derive(Debug, Clone)]
pub struct PatchCVData {
    /// New bio, or `None` to leave it alone.
    pub bio: Option<String>,
    /// New role, or `None` to leave it alone.
    pub role: Option<String>,
    /// New portrait URL, or `None` to leave it alone.
    pub photo_url: Option<String>,
    /// New display name, or `None` to leave it alone.
    pub display_name: Option<String>,
    /// Replaces the whole list when present. There is no per-item patch.
    pub core_skills: Option<Vec<CoreSkill>>,
    /// Replaces the whole list when present.
    pub educations: Option<Vec<Education>>,
    /// Replaces the whole list when present.
    pub experiences: Option<Vec<Experience>>,
    /// Replaces the whole list when present.
    pub highlighted_projects: Option<Vec<HighlightedProject>>,
    /// Replaces the whole list when present.
    pub contact_info: Option<Vec<ContactDetail>>,
}
