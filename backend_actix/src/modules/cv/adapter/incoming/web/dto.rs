//! Wire types for the CV endpoints.
//!
//! These exist so the domain entities in `cv::domain::entities` never appear on
//! the wire. Keeping them separate means the HTTP representation can carry
//! OpenAPI annotations, examples, and its own field naming without any of that
//! leaking into the innermost layer, and the two can diverge without one
//! dragging the other along.
//!
//! The value objects are symmetric — a skill looks the same going out as coming
//! in — so each is used for both request and response bodies and converts in
//! both directions.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::cv::domain::entities::{
    CVInfo, ContactDetail, ContactType, CoreSkill, Education, Experience, HighlightedProject,
};

/// Request or response shape for the HTTP layer.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CoreSkillDto {
    /// Skill title
    #[schema(example = "Backend Development")]
    pub title: String,

    /// Skill description
    #[schema(example = "Expert in Rust, Python, and Node.js")]
    pub description: String,
}

/// Request or response shape for the HTTP layer.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EducationDto {
    /// Qualification earned.
    #[schema(example = "Bachelor of Science in Computer Science")]
    pub degree: String,

    /// Where it was earned.
    #[schema(example = "MIT")]
    pub institution: String,

    /// Year of completion.
    #[schema(example = 2015)]
    pub graduation_year: i32,
}

/// Request or response shape for the HTTP layer.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExperienceDto {
    /// Employer.
    #[schema(example = "Tech Corp")]
    pub company: String,

    /// Job title.
    #[schema(example = "Senior Backend Engineer")]
    pub position: String,

    /// Where the role was based.
    #[schema(example = "San Francisco, CA")]
    pub location: String,

    /// When the role began. Free-form, so partial dates are allowed.
    #[schema(example = "2020-01")]
    pub start_date: String,

    /// Absent for a current position
    #[schema(example = "2023-12")]
    pub end_date: Option<String>,

    /// Long-form description.
    #[schema(example = "Led backend development team...")]
    pub description: String,

    /// Responsibilities, in display order.
    #[schema(example = json!(["Designed microservices architecture"]))]
    pub tasks: Vec<String>,

    /// Notable outcomes, in display order.
    #[schema(example = json!(["Reduced latency by 40%"]))]
    pub achievements: Vec<String>,
}

/// Request or response shape for the HTTP layer.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HighlightedProjectDto {
    /// The project's identifier.
    #[schema(example = "proj-123")]
    pub id: String,

    /// Display title.
    #[schema(example = "E-commerce Platform")]
    pub title: String,

    /// URL segment. Unique per owner.
    #[schema(example = "ecommerce-platform")]
    pub slug: String,

    /// One-line summary.
    #[schema(example = "A scalable e-commerce platform")]
    pub short_description: String,
}

/// See the module documentation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub enum ContactTypeDto {
    /// A telephone number.
    PhoneNumber,
    /// A link — a site, a profile, a repository.
    WebPage,
}

/// Request or response shape for the HTTP layer.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContactDetailDto {
    /// Which kind of contact row this is; decides how a client renders it.
    pub contact_type: ContactTypeDto,

    /// Display title.
    #[schema(example = "Work Email")]
    pub title: String,

    /// The body.
    #[schema(example = "john@example.com")]
    pub content: String,
}

/// A CV as returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CvResponse {
    /// Primary key.
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Uuid,

    /// The owning user.
    #[schema(example = "987e6543-e21b-12d3-a456-426614174000")]
    pub user_id: Uuid,

    /// Job title shown under the display name.
    #[schema(example = "Senior Software Engineer")]
    pub role: String,

    /// Name shown on the CV.
    #[schema(example = "John Doe")]
    pub display_name: String,

    /// Free-form introduction.
    #[schema(example = "Passionate software engineer...")]
    pub bio: String,

    /// Portrait image. Empty when unset.
    #[schema(example = "https://example.com/photos/profile.jpg")]
    pub photo_url: String,

    /// Headline skills, in display order.
    pub core_skills: Vec<CoreSkillDto>,
    /// Education entries, in display order.
    pub educations: Vec<EducationDto>,
    /// Work history, in display order.
    pub experiences: Vec<ExperienceDto>,
    /// Projects featured on the CV, in display order.
    pub highlighted_projects: Vec<HighlightedProjectDto>,
    /// Contact rows. Public on a published CV.
    pub contact_info: Vec<ContactDetailDto>,
}

//
// ──────────────────────────────────────────────────────────
// domain -> wire
// ──────────────────────────────────────────────────────────
//

impl From<CoreSkill> for CoreSkillDto {
    fn from(v: CoreSkill) -> Self {
        Self {
            title: v.title,
            description: v.description,
        }
    }
}

impl From<Education> for EducationDto {
    fn from(v: Education) -> Self {
        Self {
            degree: v.degree,
            institution: v.institution,
            graduation_year: v.graduation_year,
        }
    }
}

impl From<Experience> for ExperienceDto {
    fn from(v: Experience) -> Self {
        Self {
            company: v.company,
            position: v.position,
            location: v.location,
            start_date: v.start_date,
            end_date: v.end_date,
            description: v.description,
            tasks: v.tasks,
            achievements: v.achievements,
        }
    }
}

impl From<HighlightedProject> for HighlightedProjectDto {
    fn from(v: HighlightedProject) -> Self {
        Self {
            id: v.id,
            title: v.title,
            slug: v.slug,
            short_description: v.short_description,
        }
    }
}

impl From<ContactType> for ContactTypeDto {
    fn from(v: ContactType) -> Self {
        match v {
            ContactType::PhoneNumber => ContactTypeDto::PhoneNumber,
            ContactType::WebPage => ContactTypeDto::WebPage,
        }
    }
}

impl From<ContactDetail> for ContactDetailDto {
    fn from(v: ContactDetail) -> Self {
        Self {
            contact_type: v.contact_type.into(),
            title: v.title,
            content: v.content,
        }
    }
}

impl From<CVInfo> for CvResponse {
    fn from(cv: CVInfo) -> Self {
        Self {
            id: cv.id,
            user_id: cv.user_id,
            role: cv.role,
            display_name: cv.display_name,
            bio: cv.bio,
            photo_url: cv.photo_url,
            core_skills: cv.core_skills.into_iter().map(Into::into).collect(),
            educations: cv.educations.into_iter().map(Into::into).collect(),
            experiences: cv.experiences.into_iter().map(Into::into).collect(),
            highlighted_projects: cv
                .highlighted_projects
                .into_iter()
                .map(Into::into)
                .collect(),
            contact_info: cv.contact_info.into_iter().map(Into::into).collect(),
        }
    }
}

//
// ──────────────────────────────────────────────────────────
// wire -> domain
// ──────────────────────────────────────────────────────────
//

impl From<CoreSkillDto> for CoreSkill {
    fn from(v: CoreSkillDto) -> Self {
        Self {
            title: v.title,
            description: v.description,
        }
    }
}

impl From<EducationDto> for Education {
    fn from(v: EducationDto) -> Self {
        Self {
            degree: v.degree,
            institution: v.institution,
            graduation_year: v.graduation_year,
        }
    }
}

impl From<ExperienceDto> for Experience {
    fn from(v: ExperienceDto) -> Self {
        Self {
            company: v.company,
            position: v.position,
            location: v.location,
            start_date: v.start_date,
            end_date: v.end_date,
            description: v.description,
            tasks: v.tasks,
            achievements: v.achievements,
        }
    }
}

impl From<HighlightedProjectDto> for HighlightedProject {
    fn from(v: HighlightedProjectDto) -> Self {
        Self {
            id: v.id,
            title: v.title,
            slug: v.slug,
            short_description: v.short_description,
        }
    }
}

impl From<ContactTypeDto> for ContactType {
    fn from(v: ContactTypeDto) -> Self {
        match v {
            ContactTypeDto::PhoneNumber => ContactType::PhoneNumber,
            ContactTypeDto::WebPage => ContactType::WebPage,
        }
    }
}

impl From<ContactDetailDto> for ContactDetail {
    fn from(v: ContactDetailDto) -> Self {
        Self {
            contact_type: v.contact_type.into(),
            title: v.title,
            content: v.content,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_domain_cv() -> CVInfo {
        CVInfo {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            role: "Senior Software Engineer".into(),
            display_name: "John Doe".into(),
            bio: "Passionate engineer".into(),
            photo_url: "https://example.com/p.jpg".into(),
            core_skills: vec![CoreSkill {
                title: "Backend".into(),
                description: "Rust".into(),
            }],
            educations: vec![Education {
                degree: "BSc".into(),
                institution: "MIT".into(),
                graduation_year: 2015,
            }],
            experiences: vec![Experience {
                company: "Tech Corp".into(),
                position: "Engineer".into(),
                location: "SF".into(),
                start_date: "2020-01".into(),
                end_date: Some("2023-12".into()),
                description: "Led the team".into(),
                tasks: vec!["Designed services".into()],
                achievements: vec!["Cut latency 40%".into()],
            }],
            highlighted_projects: vec![HighlightedProject {
                id: "proj-1".into(),
                title: "Shop".into(),
                slug: "shop".into(),
                short_description: "A shop".into(),
            }],
            contact_info: vec![ContactDetail {
                contact_type: ContactType::WebPage,
                title: "Site".into(),
                content: "https://example.com".into(),
            }],
        }
    }

    /// The wire types exist to decouple layers, not to reshape data. If a
    /// conversion silently drops a field, the CV that comes back from the API
    /// is quietly missing part of what was stored, and no route test would
    /// necessarily catch it.
    #[test]
    fn cv_survives_the_trip_to_the_wire_and_back() {
        let original = sample_domain_cv();
        let response: CvResponse = original.clone().into();

        assert_eq!(response.id, original.id);
        assert_eq!(response.user_id, original.user_id);
        assert_eq!(response.role, original.role);
        assert_eq!(response.display_name, original.display_name);
        assert_eq!(response.bio, original.bio);
        assert_eq!(response.photo_url, original.photo_url);

        // Round-trip the collections back into domain types and compare.
        let skills: Vec<CoreSkill> = response.core_skills.into_iter().map(Into::into).collect();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].title, original.core_skills[0].title);
        assert_eq!(skills[0].description, original.core_skills[0].description);

        let educations: Vec<Education> = response.educations.into_iter().map(Into::into).collect();
        assert_eq!(educations[0].degree, original.educations[0].degree);
        assert_eq!(
            educations[0].institution,
            original.educations[0].institution
        );
        assert_eq!(
            educations[0].graduation_year,
            original.educations[0].graduation_year
        );

        let experiences: Vec<Experience> =
            response.experiences.into_iter().map(Into::into).collect();
        let (got, want) = (&experiences[0], &original.experiences[0]);
        assert_eq!(got.company, want.company);
        assert_eq!(got.position, want.position);
        assert_eq!(got.location, want.location);
        assert_eq!(got.start_date, want.start_date);
        assert_eq!(got.end_date, want.end_date);
        assert_eq!(got.description, want.description);
        assert_eq!(got.tasks, want.tasks);
        assert_eq!(got.achievements, want.achievements);

        let projects: Vec<HighlightedProject> = response
            .highlighted_projects
            .into_iter()
            .map(Into::into)
            .collect();
        assert_eq!(projects[0].id, original.highlighted_projects[0].id);
        assert_eq!(projects[0].slug, original.highlighted_projects[0].slug);
        assert_eq!(
            projects[0].short_description,
            original.highlighted_projects[0].short_description
        );

        let contacts: Vec<ContactDetail> =
            response.contact_info.into_iter().map(Into::into).collect();
        assert_eq!(contacts[0].contact_type, ContactType::WebPage);
        assert_eq!(contacts[0].title, original.contact_info[0].title);
        assert_eq!(contacts[0].content, original.contact_info[0].content);
    }

    #[test]
    fn contact_type_maps_both_ways_for_every_variant() {
        for (domain, wire) in [
            (ContactType::PhoneNumber, ContactTypeDto::PhoneNumber),
            (ContactType::WebPage, ContactTypeDto::WebPage),
        ] {
            let to_wire: ContactTypeDto = domain.clone().into();
            assert_eq!(to_wire, wire);

            let back: ContactType = to_wire.into();
            assert_eq!(back, domain);
        }
    }

    /// An experience with no end date represents a current position; the
    /// conversion must keep that distinct from an empty string.
    #[test]
    fn absent_end_date_stays_absent() {
        let mut cv = sample_domain_cv();
        cv.experiences[0].end_date = None;

        let response: CvResponse = cv.into();
        assert_eq!(response.experiences[0].end_date, None);
    }

    /// Empty collections must serialise as `[]`, not vanish, or clients have to
    /// handle a missing key as well as an empty list.
    #[test]
    fn empty_collections_serialise_as_empty_arrays() {
        let mut cv = sample_domain_cv();
        cv.core_skills.clear();
        cv.educations.clear();
        cv.experiences.clear();
        cv.highlighted_projects.clear();
        cv.contact_info.clear();

        let response: CvResponse = cv.into();
        let json = serde_json::to_value(&response).unwrap();

        for key in [
            "core_skills",
            "educations",
            "experiences",
            "highlighted_projects",
            "contact_info",
        ] {
            assert_eq!(json[key], serde_json::json!([]), "{key} should be []");
        }
    }
}
