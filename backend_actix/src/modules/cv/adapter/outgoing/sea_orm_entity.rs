use crate::cv::application::ports::outgoing::CreateCVData;
use crate::cv::domain::entities::CVInfo;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::Value as JsonValue;
use uuid::Uuid;

// This is the SeaORM model that directly represents the "cv" table
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "resumes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,

    pub user_id: Uuid,
    pub bio: String,
    #[sea_orm(column_type = "Text", string_len = 150)]
    pub display_name: String,
    #[sea_orm(column_type = "Text", string_len = 150)]
    pub role: String,
    pub photo_url: String,

    #[sea_orm(column_type = "JsonBinary")]
    pub core_skills: JsonValue,
    #[sea_orm(column_type = "JsonBinary")]
    pub educations: JsonValue,
    #[sea_orm(column_type = "JsonBinary")]
    pub experiences: JsonValue,
    pub highlighted_projects: JsonValue,
    #[sea_orm(column_type = "JsonBinary")]
    pub contact_info: JsonValue,

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,

    pub is_deleted: bool,
}

impl Model {
    pub fn to_domain(&self) -> CVInfo {
        CVInfo {
            id: self.id,
            user_id: self.user_id,
            role: self.role.clone(),
            display_name: self.display_name.clone(),
            bio: self.bio.clone(),
            photo_url: self.photo_url.clone(),
            core_skills: serde_json::from_value(self.core_skills.clone()).unwrap_or_default(),
            educations: serde_json::from_value(self.educations.clone()).unwrap_or_default(),
            experiences: serde_json::from_value(self.experiences.clone()).unwrap_or_default(),
            highlighted_projects: serde_json::from_value(self.highlighted_projects.clone())
                .unwrap_or_default(),
            contact_info: serde_json::from_value(self.contact_info.clone()).unwrap_or_default(),
        }
    }
    pub fn from_create_data(user_id: Uuid, cv: &CreateCVData) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            role: cv.role.clone(),
            display_name: cv.display_name.clone(),
            bio: cv.bio.clone(),
            photo_url: cv.photo_url.clone(),
            core_skills: serde_json::to_value(&cv.core_skills).unwrap(),
            educations: serde_json::to_value(&cv.educations).unwrap(),
            experiences: serde_json::to_value(&cv.experiences).unwrap(),
            highlighted_projects: serde_json::to_value(&cv.highlighted_projects).unwrap(),
            contact_info: serde_json::to_value(&cv.contact_info).unwrap(),
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
            is_deleted: false,
        }
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
