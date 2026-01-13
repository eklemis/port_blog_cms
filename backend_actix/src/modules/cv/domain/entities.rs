use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CVInfo {
    pub id: String,
    pub role: String,
    pub bio: String,
    pub photo_url: String,
    pub core_skills: Vec<CoreSkill>,
    pub educations: Vec<Education>,
    pub experiences: Vec<Experience>,
    pub highlighted_projects: Vec<HighlightedProject>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CoreSkill {
    pub title: String,
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
    pub start_date: String, // For simplicity; or consider chrono::NaiveDate
    pub end_date: Option<String>,
    pub description: String,
    pub tasks: Vec<String>,
    pub achievements: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HighlightedProject {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub short_description: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub tech_stack: Vec<String>,
    pub featured: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SingleProjectDetails {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub tech_stack: Vec<String>,
    pub featured: bool,
    pub screenshots: Vec<Screenshot>,
    pub repo_url: String,
    pub live_demo_url: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Screenshot {
    pub url: String,
    pub order: i32,
    pub featured: bool,
}
