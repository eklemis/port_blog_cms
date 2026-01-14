// cv_repository.rs
use crate::cv::domain::entities::{CVInfo, CoreSkill, Education, Experience, HighlightedProject};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug)]
pub enum CVRepositoryError {
    NotFound,
    DatabaseError(String),
}

#[async_trait]
pub trait CVRepository: Send + Sync {
    async fn fetch_cv_by_user_id(&self, user_id: Uuid) -> Result<Vec<CVInfo>, CVRepositoryError>;
    async fn create_cv(
        &self,
        user_id: Uuid,
        cv_data: CreateCVData,
    ) -> Result<CVInfo, CVRepositoryError>;
    async fn update_cv(
        &self,
        cv_id: Uuid,
        cv_data: UpdateCVData,
    ) -> Result<CVInfo, CVRepositoryError>;
}

// Separate struct for creating CV (no ID needed from user)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCVData {
    pub role: String,
    pub bio: String,
    pub photo_url: String,
    pub core_skills: Vec<CoreSkill>,
    pub educations: Vec<Education>,
    pub experiences: Vec<Experience>,
    pub highlighted_projects: Vec<HighlightedProject>,
}

// Separate struct for updating CV
pub type UpdateCVData = CreateCVData;
