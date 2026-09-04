//! Gathers a CV, a posting and any existing letter for a drafting pass.
//!
//! `ai` declares what it needs; this fetches it from the modules that own it.
//! The rendering to prose happens here rather than in the service, because how
//! a CV reads is a property of a CV.

use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::ai::application::ports::outgoing::{
    DraftingContextError, DraftingContextReader, DraftingMaterial,
};
use crate::career::application::ports::outgoing::{ApplicationStore, JobStore, LetterStore};
use crate::cv::application::ports::outgoing::{CVQuery, CvSnapshotStore};
use crate::cv::domain::entities::CVInfo;

/// Renders a CV as the prose a model reads.
///
/// Deliberately plain. A model does not need the JSON, and giving it field
/// names invites it to answer in field names.
fn render(cv: &CVInfo) -> String {
    let mut out = format!("{}\n{}\n\n{}", cv.display_name, cv.role, cv.bio);

    if !cv.core_skills.is_empty() {
        out.push_str("\n\nSkills\n");
        for s in &cv.core_skills {
            out.push_str(&format!("- {}: {}\n", s.title, s.description));
        }
    }

    if !cv.experiences.is_empty() {
        out.push_str("\nExperience\n");
        for e in &cv.experiences {
            out.push_str(&format!(
                "- {} at {} ({} to {})\n  {}\n",
                e.position,
                e.company,
                e.start_date,
                e.end_date.as_deref().unwrap_or("present"),
                e.description
            ));
            for task in e.tasks.iter().chain(e.achievements.iter()) {
                out.push_str(&format!("  - {task}\n"));
            }
        }
    }

    if !cv.educations.is_empty() {
        out.push_str("\nEducation\n");
        for e in &cv.educations {
            out.push_str(&format!(
                "- {}, {} ({})\n",
                e.degree, e.institution, e.graduation_year
            ));
        }
    }

    out
}

/// Reads the material from the career and cv modules.
pub struct DraftingContextCareer<A, J, L, Q> {
    applications: A,
    jobs: J,
    letters: L,
    cvs: Q,
    snapshots: Arc<dyn CvSnapshotStore>,
}

impl<A, J, L, Q> DraftingContextCareer<A, J, L, Q> {
    /// Builds it from the ports it depends on.
    pub fn new(
        applications: A,
        jobs: J,
        letters: L,
        cvs: Q,
        snapshots: Arc<dyn CvSnapshotStore>,
    ) -> Self {
        Self {
            applications,
            jobs,
            letters,
            cvs,
            snapshots,
        }
    }
}

#[async_trait]
impl<A, J, L, Q> DraftingContextReader for DraftingContextCareer<A, J, L, Q>
where
    A: ApplicationStore + Send + Sync,
    J: JobStore + Send + Sync,
    L: LetterStore + Send + Sync,
    Q: CVQuery + Send + Sync,
{
    async fn load(
        &self,
        owner: Uuid,
        application_id: Uuid,
        cv_id: Option<Uuid>,
    ) -> Result<DraftingMaterial, DraftingContextError> {
        let failed = |e: String| DraftingContextError::Failed(e);

        let application = self
            .applications
            .find(owner, application_id)
            .await
            .map_err(|e| failed(e.to_string()))?
            .ok_or(DraftingContextError::NotFound)?;

        let job = self
            .jobs
            .find(owner, application.job_id)
            .await
            .map_err(|e| failed(e.to_string()))?
            .ok_or(DraftingContextError::NotFound)?;

        // A named CV wins over the snapshot: tailoring is about the version
        // being worked on. Once sent, the snapshot is the only honest thing to
        // read — the same rule the analysis endpoint follows.
        let cv = match (cv_id, application.cv_snapshot_id) {
            (Some(id), _) => self
                .cvs
                .fetch_cv_by_id(id)
                .await
                .map_err(|e| failed(e.to_string()))?
                // fetch_cv_by_id takes an id and nothing else, so ownership is
                // checked here — otherwise naming someone else's cv_id would
                // read their CV into a prompt.
                .filter(|cv| cv.user_id == owner)
                .ok_or(DraftingContextError::NoCv)?,

            (None, Some(snapshot_id)) => self
                .snapshots
                .find(owner, snapshot_id)
                .await
                .map_err(|e| failed(e.to_string()))?
                .map(|s| s.document)
                .ok_or(DraftingContextError::NoCv)?,

            (None, None) => return Err(DraftingContextError::NoCv),
        };

        // The posting verbatim where one was kept, falling back to what was
        // extracted. The original is what the employer actually published.
        let job_text = if job.source_text.trim().is_empty() {
            format!(
                "{} at {}\n{}\nRequires: {}",
                job.title,
                job.company,
                job.location,
                job.required_skills.join(", ")
            )
        } else {
            job.source_text
        };

        let existing_letter = self
            .letters
            .find_letter(owner, application_id)
            .await
            .map_err(|e| failed(e.to_string()))?
            .map(|l| l.content);

        Ok(DraftingMaterial {
            cv: render(&cv),
            job: job_text,
            existing_letter,
        })
    }
}
