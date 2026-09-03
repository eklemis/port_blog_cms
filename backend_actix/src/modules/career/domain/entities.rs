//! What the Career Studio is about: postings, and what happened when you
//! applied to them.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Where an application has got to.
///
/// Server-side rather than a free string, because the whole feature is
/// pattern-finding over these values and a typo would quietly split a stage in
/// two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApplicationStatus {
    /// Being prepared. Has not been sent.
    #[default]
    Draft,
    /// Sent.
    Applied,
    /// Someone is reading it.
    Screening,
    /// Interviewing.
    Interview,
    /// Final stage.
    Final,
    /// An offer was made.
    Offer,
    /// The offer was accepted.
    Accepted,
    /// Turned down by them.
    Rejected,
    /// Withdrawn by the applicant.
    Withdrawn,
    /// **Never answered.**
    ///
    /// Deliberately distinct from [`Rejected`](Self::Rejected). Silence is the
    /// most common outcome and the most informative one: a rejection after
    /// three interviews and a posting that never replied are different events
    /// with different lessons, and folding them together destroys the only
    /// pattern worth surfacing later.
    NoReply,
}

impl ApplicationStatus {
    /// True while the application has not been sent.
    ///
    /// The transition out of this state is what triggers the CV snapshot, so
    /// it is asked about by name rather than compared inline.
    pub fn is_draft(&self) -> bool {
        matches!(self, ApplicationStatus::Draft)
    }

    /// Every status, for validation and documentation.
    pub const ALL: &'static [ApplicationStatus] = &[
        ApplicationStatus::Draft,
        ApplicationStatus::Applied,
        ApplicationStatus::Screening,
        ApplicationStatus::Interview,
        ApplicationStatus::Final,
        ApplicationStatus::Offer,
        ApplicationStatus::Accepted,
        ApplicationStatus::Rejected,
        ApplicationStatus::Withdrawn,
        ApplicationStatus::NoReply,
    ];
}

impl fmt::Display for ApplicationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Must match the CHECK constraint in the migration, and the serde
        // form above. One string per status, everywhere.
        let s = match self {
            ApplicationStatus::Draft => "draft",
            ApplicationStatus::Applied => "applied",
            ApplicationStatus::Screening => "screening",
            ApplicationStatus::Interview => "interview",
            ApplicationStatus::Final => "final",
            ApplicationStatus::Offer => "offer",
            ApplicationStatus::Accepted => "accepted",
            ApplicationStatus::Rejected => "rejected",
            ApplicationStatus::Withdrawn => "withdrawn",
            ApplicationStatus::NoReply => "no_reply",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for ApplicationStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .find(|v| v.to_string() == s)
            .copied()
            .ok_or(())
    }
}

/// A posting, kept as found plus whatever was extracted from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    /// Identifier.
    pub id: Uuid,
    /// Owner.
    pub user_id: Uuid,
    /// Role title.
    pub title: String,
    /// Hiring company.
    pub company: String,
    /// Where the role is. Empty when unstated.
    pub location: String,
    /// Seniority as advertised. Empty when unstated.
    pub seniority: String,
    /// Extracted must-haves.
    pub required_skills: Vec<String>,
    /// Extracted nice-to-haves.
    pub nice_to_have: Vec<String>,
    /// Where it was found. Empty when pasted rather than linked.
    pub source_url: String,
    /// **The posting verbatim.**
    ///
    /// Kept because postings get taken down, and at interview time this is the
    /// only record of what was actually asked for. Everything else on this
    /// struct is derived from it and can be re-derived; this cannot be
    /// recovered.
    pub source_text: String,
    /// When it was captured.
    pub created_at: DateTime<Utc>,
    /// Last edit.
    pub updated_at: DateTime<Utc>,
}

/// One application to one job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Application {
    /// Identifier.
    pub id: Uuid,
    /// Owner.
    pub user_id: Uuid,
    /// The posting applied to.
    pub job_id: Uuid,
    /// The frozen CV that was sent.
    ///
    /// `None` only while the application is a draft — leaving draft requires
    /// one, because a row without a snapshot is a row that lies later.
    pub cv_snapshot_id: Option<Uuid>,
    /// Where it has got to.
    pub status: ApplicationStatus,
    /// When it was sent. `None` while still a draft.
    pub applied_at: Option<DateTime<Utc>>,
    /// What the applicant owes it next, in their own words.
    pub next_action: String,
    /// When that is due.
    pub next_action_at: Option<DateTime<Utc>>,
    /// When the row was created.
    pub created_at: DateTime<Utc>,
    /// Last edit.
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    /// The wire form, the stored form and the CHECK constraint are the same
    /// string. A mismatch would be a row the database rejects or a status the
    /// client cannot name.
    #[test]
    fn every_status_round_trips_through_its_wire_form() {
        for status in ApplicationStatus::ALL {
            let wire = status.to_string();

            assert_eq!(
                ApplicationStatus::from_str(&wire),
                Ok(*status),
                "{status:?} did not round-trip"
            );
            assert_eq!(
                serde_json::to_string(status).unwrap(),
                format!("\"{wire}\""),
                "serde and Display disagree for {status:?}"
            );
        }
    }

    /// The distinction the tracker exists to preserve.
    #[test]
    fn silence_is_not_rejection() {
        assert_ne!(ApplicationStatus::NoReply, ApplicationStatus::Rejected);
        assert_eq!(ApplicationStatus::NoReply.to_string(), "no_reply");
    }

    #[test]
    fn only_draft_is_a_draft() {
        assert!(ApplicationStatus::Draft.is_draft());
        for status in ApplicationStatus::ALL
            .iter()
            .filter(|s| **s != ApplicationStatus::Draft)
        {
            assert!(!status.is_draft(), "{status:?} must not count as a draft");
        }
    }
}
