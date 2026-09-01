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

/// A CV in full, as the domain sees it.
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

/// A headline skill shown near the top of a CV.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CoreSkill {
    /// Skill title/name
    pub title: String,

    /// Skill description
    pub description: String,
}

/// One education entry.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Education {
    /// Qualification earned.
    pub degree: String,
    /// Where it was earned.
    pub institution: String,
    /// Year of completion.
    pub graduation_year: i32,
}

/// One role in the work history.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Experience {
    /// Employer.
    pub company: String,
    /// Job title.
    pub position: String,
    /// Where the role was based.
    pub location: String,
    /// When the role began. Free-form text, so partial dates like `2019` are
    /// allowed.
    pub start_date: String,
    /// When it ended. `None` means current.
    pub end_date: Option<String>,
    /// Free-form summary of the role.
    pub description: String,
    /// Responsibilities, in display order.
    pub tasks: Vec<String>,
    /// Notable outcomes, in display order.
    pub achievements: Vec<String>,
}

// INTENTION NOT CLEAR
/// A project featured on a CV. Carries only what the CV renders, not the
/// whole project.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HighlightedProject {
    /// The project's identifier.
    pub id: String,
    /// Project title.
    pub title: String,
    /// URL segment, for linking through to the project.
    pub slug: String,
    /// One-line summary shown on the CV.
    pub short_description: String,
}

/// A project as the CV module sees it.
#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
    /// The user who owns it.
    pub owner: UserId,
    /// The project's identifier.
    pub id: String,
    /// Project title.
    pub title: String,
    /// URL segments. A list because a renamed project keeps its old ones
    /// resolvable.
    pub slugs: Vec<String>,
    /// Free-form summary of the role.
    pub description: String,
    /// Technology labels, in display order.
    pub tech_stack: Vec<String>,
    /// Images, ordered by their `order` field.
    pub screenshots: Vec<Screenshot>,
    /// Source repository. Empty when none was set.
    pub repo_url: String,
    /// Running instance. Empty when none was set.
    pub live_demo_url: String,
    /// Topics attached to the project.
    pub project_topics: Vec<Topic>,
    /// When the project was created.
    pub created_at: String,
    /// When it was last edited.
    pub updated_at: String,
}

/// One project image.
#[derive(Serialize, Deserialize, Debug)]
pub struct Screenshot {
    /// Where the image is served from.
    pub url: String,
    /// Display position, ascending.
    pub order: i32,
    /// Featured images sort ahead of the rest regardless of `order`.
    pub featured: bool,
}

/// What kind of contact row this is, which decides how it renders.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ContactType {
    /// A telephone number.
    PhoneNumber,
    /// A link — a site, a profile, a repository.
    WebPage,
}

/// One contact row on a CV. Public on a published CV.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContactDetail {
    /// Type of contact
    pub contact_type: ContactType,

    /// Contact title/label
    pub title: String,

    /// Contact value
    pub content: String,
}
