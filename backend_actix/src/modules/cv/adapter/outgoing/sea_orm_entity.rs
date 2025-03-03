use crate::cv::domain::entities::{CVInfo, Education, Experience, HighlightedProject};
use sea_orm::entity::prelude::*;
use serde_json;
use serde_json::Value as JsonValue;
use uuid::Uuid;

// This is the SeaORM model that directly represents the "cv" table
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "cv")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub user_id: Uuid,

    pub bio: String,
    pub photo_url: String,

    pub educations_json: JsonValue,
    pub experiences_json: JsonValue,
    pub highlighted_projects_json: JsonValue,

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

impl Model {
    pub fn to_domain(&self) -> CVInfo {
        // If you stored these as JSON arrays/objects:
        let educations: Vec<Education> =
            serde_json::from_value(self.educations_json.clone()).unwrap_or_default();
        let experiences: Vec<Experience> =
            serde_json::from_value(self.experiences_json.clone()).unwrap_or_default();
        let highlighted_projects: Vec<HighlightedProject> =
            serde_json::from_value(self.highlighted_projects_json.clone()).unwrap_or_default();

        CVInfo {
            bio: self.bio.clone(),
            photo_url: self.photo_url.clone(),
            educations,
            experiences,
            highlighted_projects,
            // etc.
        }
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
