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
    NotFound,
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
    pub role: String,
    pub bio: String,
    pub display_name: String,
    pub photo_url: String,
    pub core_skills: Vec<CoreSkill>,
    pub educations: Vec<Education>,
    pub experiences: Vec<Experience>,
    pub highlighted_projects: Vec<HighlightedProject>,
    pub contact_info: Vec<ContactDetail>,
}

// Separate struct for updating CV
pub type UpdateCVData = CreateCVData;

/// A partial CV update. Omitted fields are left as stored.
#[derive(Debug, Clone)]
pub struct PatchCVData {
    pub bio: Option<String>,
    pub role: Option<String>,
    pub photo_url: Option<String>,
    pub display_name: Option<String>,
    pub core_skills: Option<Vec<CoreSkill>>,
    pub educations: Option<Vec<Education>>,
    pub experiences: Option<Vec<Experience>>,
    pub highlighted_projects: Option<Vec<HighlightedProject>>,
    pub contact_info: Option<Vec<ContactDetail>>,
}
