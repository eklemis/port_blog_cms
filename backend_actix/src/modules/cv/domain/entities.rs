//! CV domain entities.
//!
//! These are deliberately free of `utoipa` annotations and any other HTTP
//! concern. The wire representation lives in
//! `cv::adapter::incoming::web::dto`, which converts to and from these types,
//! so the API shape can change without touching the domain and vice versa.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::application::domain::entities::UserId,
    modules::topic::application::domain::entities::Topic,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CVInfo {
    /// CV unique identifier
    pub id: Uuid,

    /// Owner user ID
    pub user_id: Uuid,

    /// Professional role
    pub role: String,

    /// Display name
    pub display_name: String,

    /// Biography
    pub bio: String,

    /// Profile photo URL
    pub photo_url: String,

    /// Core skills
    pub core_skills: Vec<CoreSkill>,

    /// Educational background
    pub educations: Vec<Education>,

    /// Work experiences
    pub experiences: Vec<Experience>,

    /// Highlighted projects
    pub highlighted_projects: Vec<HighlightedProject>,

    /// Contact information
    pub contact_info: Vec<ContactDetail>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CoreSkill {
    /// Skill title/name
    pub title: String,

    /// Skill description
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Education {
    pub degree: String,
    pub institution: String,
    pub graduation_year: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Experience {
    pub company: String,
    pub position: String,
    pub location: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub description: String,
    pub tasks: Vec<String>,
    pub achievements: Vec<String>,
}

// INTENTION NOT CLEAR
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HighlightedProject {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub short_description: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
    pub owner: UserId,
    pub id: String,
    pub title: String,
    pub slugs: Vec<String>,
    pub description: String,
    pub tech_stack: Vec<String>,
    pub screenshots: Vec<Screenshot>,
    pub repo_url: String,
    pub live_demo_url: String,
    pub project_topics: Vec<Topic>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Screenshot {
    pub url: String,
    pub order: i32,
    pub featured: bool, //shows first
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ContactType {
    PhoneNumber,
    WebPage,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContactDetail {
    /// Type of contact
    pub contact_type: ContactType,

    /// Contact title/label
    pub title: String,

    /// Contact value
    pub content: String,
}
