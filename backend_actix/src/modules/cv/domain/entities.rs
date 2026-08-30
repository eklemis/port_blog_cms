use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::application::domain::entities::UserId,
    modules::topic::application::domain::entities::Topic,
};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct CVInfo {
    /// CV unique identifier
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Uuid,

    /// Owner user ID
    #[schema(example = "987e6543-e21b-12d3-a456-426614174000")]
    pub user_id: Uuid,

    /// Professional role
    #[schema(example = "Senior Software Engineer")]
    pub role: String,

    /// Display name
    #[schema(example = "John Doe")]
    pub display_name: String,

    /// Biography
    #[schema(example = "Passionate software engineer...")]
    pub bio: String,

    /// Profile photo URL
    #[schema(example = "https://example.com/photos/profile.jpg")]
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

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct CoreSkill {
    /// Skill title/name
    #[schema(example = "Backend Development")]
    pub title: String,

    /// Skill description
    #[schema(example = "Expert in Rust, Python, and Node.js")]
    pub description: String,
}

// Education, Experience, and HighlightedProject also need ToSchema
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Education {
    #[schema(example = "Bachelor of Science in Computer Science")]
    pub degree: String,

    #[schema(example = "MIT")]
    pub institution: String,

    #[schema(example = 2015)]
    pub graduation_year: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Experience {
    #[schema(example = "Tech Corp")]
    pub company: String,

    #[schema(example = "Senior Backend Engineer")]
    pub position: String,

    #[schema(example = "San Francisco, CA")]
    pub location: String,

    #[schema(example = "2020-01")]
    pub start_date: String,

    #[schema(example = "2023-12")]
    pub end_date: Option<String>,

    #[schema(example = "Led backend development team...")]
    pub description: String,

    pub tasks: Vec<String>,

    pub achievements: Vec<String>,
}

// INTENTION NOT CLEAR
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct HighlightedProject {
    #[schema(example = "proj-123")]
    pub id: String,

    #[schema(example = "E-commerce Platform")]
    pub title: String,

    #[schema(example = "ecommerce-platform")]
    pub slug: String,

    #[schema(example = "A scalable e-commerce platform...")]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub enum ContactType {
    PhoneNumber,
    WebPage,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ContactDetail {
    /// Type of contact
    pub contact_type: ContactType,

    /// Contact title/label
    #[schema(example = "Work Email")]
    pub title: String,

    /// Contact value
    #[schema(example = "john@example.com")]
    pub content: String,
}
