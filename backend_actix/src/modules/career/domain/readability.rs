//! The measurable half of match analysis.
//!
//! Deterministic, and computed here rather than estimated by a model. That
//! split is the design: a single blended score would hide which half a reader
//! should trust, and these checks are the half that can be *right* rather than
//! merely plausible.
//!
//! # What this can and cannot see
//!
//! A CV in this product is structured data, not a rendered document. That
//! makes some checks sharper than they would be against a PDF — dates are
//! stored as free text, so "do the dates parse" is a real question with a real
//! answer — and makes others impossible. **Layout is not visible here.**
//! Whether a CV renders as one column or two is decided by the template that
//! draws it, so a `single_column` check would be the backend guessing about
//! the frontend's rendering. It is deliberately absent rather than faked.

use crate::cv::domain::entities::CVInfo;

/// One thing that was checked, and how it came out.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct ReadabilityCheck {
    /// Stable identifier. The wire contract — clients key their copy off this,
    /// so it must not change once shipped.
    pub id: String,

    /// Whether it passed.
    pub ok: bool,

    /// What was wrong, when something was. Prose, and may change; it names the
    /// offending entry so a person can go and fix it.
    pub detail: Option<String>,
}

impl ReadabilityCheck {
    fn pass(id: &str) -> Self {
        Self {
            id: id.to_string(),
            ok: true,
            detail: None,
        }
    }

    fn fail(id: &str, detail: impl Into<String>) -> Self {
        Self {
            id: id.to_string(),
            ok: false,
            detail: Some(detail.into()),
        }
    }
}

/// The measured half of an analysis.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct ReadabilityReport {
    /// Percentage of checks that passed, rounded.
    ///
    /// Arithmetic over [`checks`](Self::checks) and nothing else, so a reader
    /// can always reconstruct it from what is shown. It is **not** blended
    /// with the model's relevance estimate — see the module documentation.
    pub score: u8,

    /// Every check, passed and failed, in a stable order.
    ///
    /// Passes are included deliberately: a list that only showed problems
    /// would leave a person unable to tell "nothing was wrong" from "nothing
    /// was looked at".
    pub checks: Vec<ReadabilityCheck>,
}

/// Reads a date the way a person would have typed it.
///
/// Accepts `YYYY`, `YYYY-MM` and `YYYY-MM-DD`, plus the common `MM/YYYY`. This
/// is intentionally generous: the field is free text on an existing CV, and the
/// check is meant to catch "Summer, sort of" rather than to impose a format
/// nobody was told about.
fn parses_as_date(raw: &str) -> bool {
    let s = raw.trim();
    if s.is_empty() {
        return false;
    }
    if s.eq_ignore_ascii_case("present") || s.eq_ignore_ascii_case("current") {
        return true;
    }

    let numeric_parts = |sep: char, expected: usize| -> bool {
        let parts: Vec<&str> = s.split(sep).collect();
        parts.len() == expected
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
    };

    numeric_parts('-', 1) && s.len() == 4
        || numeric_parts('-', 2)
        || numeric_parts('-', 3)
        || numeric_parts('/', 2)
}

/// The year part of a date this function already considers parseable.
fn year_of(raw: &str) -> Option<i32> {
    let s = raw.trim();
    let candidate = if s.contains('/') {
        s.split('/').next_back()?
    } else {
        s.split('-').next()?
    };
    candidate.parse().ok()
}

/// Runs every check against a CV.
///
/// Pure: no I/O, no clock beyond the year passed in. `this_year` is a
/// parameter rather than read from the clock so the plausibility check is
/// testable and cannot drift.
pub fn assess(cv: &CVInfo, this_year: i32) -> ReadabilityReport {
    let mut checks = Vec::new();

    // ── Is there anything to read at all ────────────────────────────────
    checks.push(if cv.experiences.is_empty() {
        ReadabilityCheck::fail(
            "has_experience",
            "No work history. Most screens filter on it before a person reads anything.",
        )
    } else {
        ReadabilityCheck::pass("has_experience")
    });

    checks.push(if cv.core_skills.is_empty() {
        ReadabilityCheck::fail(
            "has_skills",
            "No skills listed. Keyword matching has nothing to match against.",
        )
    } else {
        ReadabilityCheck::pass("has_skills")
    });

    checks.push(
        if cv.contact_info.iter().any(|c| !c.content.trim().is_empty()) {
            ReadabilityCheck::pass("has_contact")
        } else {
            ReadabilityCheck::fail(
                "has_contact",
                "No way to reply. Everything else on the CV is wasted without one.",
            )
        },
    );

    // ── Do the dates say what they mean ─────────────────────────────────
    let unparseable: Vec<String> = cv
        .experiences
        .iter()
        .filter(|e| !parses_as_date(&e.start_date))
        .map(|e| format!("{} at {}", e.position, e.company))
        .collect();

    checks.push(if unparseable.is_empty() {
        ReadabilityCheck::pass("dates_parse")
    } else {
        ReadabilityCheck::fail(
            "dates_parse",
            format!(
                "Start date not recognised on: {}. Use a year, or YYYY-MM.",
                unparseable.join("; ")
            ),
        )
    });

    let out_of_order: Vec<String> = cv
        .experiences
        .iter()
        .filter_map(|e| {
            let end = e.end_date.as_ref()?;
            let (start_year, end_year) = (year_of(&e.start_date)?, year_of(end)?);
            (end_year < start_year).then(|| format!("{} at {}", e.position, e.company))
        })
        .collect();

    checks.push(if out_of_order.is_empty() {
        ReadabilityCheck::pass("dates_ordered")
    } else {
        ReadabilityCheck::fail(
            "dates_ordered",
            format!("Ends before it starts: {}", out_of_order.join("; ")),
        )
    });

    let implausible: Vec<String> = cv
        .educations
        .iter()
        .filter(|e| {
            e.graduation_year != 0
                && (e.graduation_year < 1900 || e.graduation_year > this_year + 10)
        })
        .map(|e| format!("{} ({})", e.degree, e.graduation_year))
        .collect();

    checks.push(if implausible.is_empty() {
        ReadabilityCheck::pass("graduation_years_plausible")
    } else {
        ReadabilityCheck::fail(
            "graduation_years_plausible",
            format!("Graduation year looks wrong: {}", implausible.join("; ")),
        )
    });

    // ── Is every entry actually filled in ───────────────────────────────
    let unnamed: Vec<String> = cv
        .experiences
        .iter()
        .enumerate()
        .filter(|(_, e)| e.position.trim().is_empty() || e.company.trim().is_empty())
        .map(|(i, _)| format!("entry {}", i + 1))
        .collect();

    checks.push(if unnamed.is_empty() {
        ReadabilityCheck::pass("roles_named")
    } else {
        ReadabilityCheck::fail(
            "roles_named",
            format!(
                "Missing a job title or a company on: {}",
                unnamed.join(", ")
            ),
        )
    });

    let undescribed: Vec<String> = cv
        .experiences
        .iter()
        .filter(|e| {
            e.description.trim().is_empty() && e.tasks.is_empty() && e.achievements.is_empty()
        })
        .map(|e| format!("{} at {}", e.position, e.company))
        .collect();

    checks.push(if undescribed.is_empty() {
        ReadabilityCheck::pass("roles_described")
    } else {
        ReadabilityCheck::fail(
            "roles_described",
            format!(
                "A job title with nothing under it says very little: {}",
                undescribed.join("; ")
            ),
        )
    });

    checks.push(if cv.role.trim().is_empty() {
        ReadabilityCheck::fail(
            "headline_present",
            "No headline role. It is the first line anyone reads.",
        )
    } else {
        ReadabilityCheck::pass("headline_present")
    });

    let passed = checks.iter().filter(|c| c.ok).count();
    let score = ((passed as f32 / checks.len() as f32) * 100.0).round() as u8;

    ReadabilityReport { score, checks }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cv::domain::entities::{
        ContactDetail, ContactType, CoreSkill, Education, Experience,
    };
    use uuid::Uuid;

    fn an_experience(start: &str, end: Option<&str>) -> Experience {
        Experience {
            company: "Acme".into(),
            position: "Engineer".into(),
            location: "Remote".into(),
            start_date: start.into(),
            end_date: end.map(str::to_string),
            description: "Built things".into(),
            tasks: vec![],
            achievements: vec![],
        }
    }

    fn a_good_cv() -> CVInfo {
        CVInfo {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            role: "Backend Engineer".into(),
            display_name: "Jane Doe".into(),
            bio: "Rust, mostly.".into(),
            photo_url: String::new(),
            core_skills: vec![CoreSkill {
                title: "Rust".into(),
                description: "Five years".into(),
            }],
            educations: vec![Education {
                degree: "BSc Computer Science".into(),
                institution: "A University".into(),
                graduation_year: 2018,
            }],
            experiences: vec![an_experience("2020-01", Some("2024-06"))],
            highlighted_projects: vec![],
            contact_info: vec![ContactDetail {
                contact_type: ContactType::WebPage,
                title: "Email".into(),
                content: "jane@example.com".into(),
            }],
        }
    }

    fn check<'a>(report: &'a ReadabilityReport, id: &str) -> &'a ReadabilityCheck {
        report
            .checks
            .iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("no check called {id}"))
    }

    #[test]
    fn a_complete_cv_passes_everything() {
        let report = assess(&a_good_cv(), 2026);

        assert_eq!(report.score, 100);
        assert!(report.checks.iter().all(|c| c.ok));
    }

    /// Passes are reported, not just failures. A list of only problems leaves
    /// a reader unable to tell "nothing wrong" from "nothing checked".
    #[test]
    fn passing_checks_are_still_reported() {
        let report = assess(&a_good_cv(), 2026);

        assert!(report.checks.len() >= 8);
        assert!(report.checks.iter().all(|c| c.detail.is_none()));
    }

    /// The score is arithmetic over the checks and nothing else, so a reader
    /// can reconstruct it from what they are shown.
    #[test]
    fn the_score_is_the_proportion_that_passed() {
        let mut cv = a_good_cv();
        cv.core_skills.clear();
        cv.contact_info.clear();

        let report = assess(&cv, 2026);
        let passed = report.checks.iter().filter(|c| c.ok).count();
        let expected = ((passed as f32 / report.checks.len() as f32) * 100.0).round() as u8;

        assert_eq!(report.score, expected);
        assert!(report.score < 100);
    }

    // ── dates ──────────────────────────────────────────────────────────

    #[test]
    fn a_vague_start_date_is_caught() {
        let mut cv = a_good_cv();
        cv.experiences = vec![an_experience("sometime in 2020", None)];

        let report = assess(&cv, 2026);
        let result = check(&report, "dates_parse");

        assert!(!result.ok);
        assert!(
            result.detail.as_ref().unwrap().contains("Engineer at Acme"),
            "the detail must name the entry so it can be fixed"
        );
    }

    /// Generous on purpose: the field is free text on CVs that already exist,
    /// and the check is meant to catch prose, not to impose a format.
    #[test]
    fn the_ordinary_ways_of_writing_a_date_all_parse() {
        for form in [
            "2020",
            "2020-01",
            "2020-01-15",
            "01/2020",
            "Present",
            "current",
        ] {
            let mut cv = a_good_cv();
            cv.experiences = vec![an_experience(form, None)];

            assert!(
                check(&assess(&cv, 2026), "dates_parse").ok,
                "{form} should be accepted"
            );
        }
    }

    #[test]
    fn a_job_that_ends_before_it_starts_is_caught() {
        let mut cv = a_good_cv();
        cv.experiences = vec![an_experience("2024-01", Some("2020-01"))];

        assert!(!check(&assess(&cv, 2026), "dates_ordered").ok);
    }

    #[test]
    fn an_ongoing_job_is_not_out_of_order() {
        let mut cv = a_good_cv();
        cv.experiences = vec![an_experience("2024-01", Some("Present"))];

        assert!(check(&assess(&cv, 2026), "dates_ordered").ok);
    }

    /// `this_year` is a parameter so this check cannot start failing on its
    /// own as the clock moves.
    #[test]
    fn a_graduation_year_far_in_the_future_is_caught() {
        let mut cv = a_good_cv();
        cv.educations[0].graduation_year = 2099;

        assert!(!check(&assess(&cv, 2026), "graduation_years_plausible").ok);
    }

    /// A degree in progress is commonly stored with no year at all. That is
    /// not the same as a wrong one.
    #[test]
    fn an_unset_graduation_year_is_not_treated_as_wrong() {
        let mut cv = a_good_cv();
        cv.educations[0].graduation_year = 0;

        assert!(check(&assess(&cv, 2026), "graduation_years_plausible").ok);
    }

    // ── completeness ───────────────────────────────────────────────────

    #[test]
    fn an_empty_cv_fails_loudly_rather_than_scoring_well() {
        let mut cv = a_good_cv();
        cv.experiences.clear();
        cv.core_skills.clear();
        cv.contact_info.clear();
        cv.role = String::new();

        let report = assess(&cv, 2026);

        assert!(!check(&report, "has_experience").ok);
        assert!(!check(&report, "has_skills").ok);
        assert!(!check(&report, "has_contact").ok);
        assert!(!check(&report, "headline_present").ok);
        assert!(report.score < 60);
    }

    #[test]
    fn a_contact_row_with_no_content_does_not_count() {
        let mut cv = a_good_cv();
        cv.contact_info[0].content = "   ".into();

        assert!(!check(&assess(&cv, 2026), "has_contact").ok);
    }

    #[test]
    fn a_job_with_no_description_is_caught() {
        let mut cv = a_good_cv();
        cv.experiences[0].description = String::new();

        assert!(!check(&assess(&cv, 2026), "roles_described").ok);
    }

    /// Tasks or achievements count as description — plenty of CVs use bullets
    /// instead of a paragraph, and that is not a defect.
    #[test]
    fn bullets_count_as_a_description() {
        let mut cv = a_good_cv();
        cv.experiences[0].description = String::new();
        cv.experiences[0].tasks = vec!["Ran the deploy pipeline".into()];

        assert!(check(&assess(&cv, 2026), "roles_described").ok);
    }

    /// Check ids are the wire contract: clients key their own copy off them,
    /// so a rename is a breaking change and a duplicate is ambiguous.
    #[test]
    fn check_ids_are_unique() {
        let report = assess(&a_good_cv(), 2026);
        let mut ids: Vec<&str> = report.checks.iter().map(|c| c.id.as_str()).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();

        assert_eq!(ids.len(), before, "duplicate check id");
    }
}
