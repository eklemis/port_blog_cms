//! The estimated half of match analysis.
//!
//! A model's judgement about how well a CV answers one job. Reported entirely
//! separately from [`readability`](crate::career::domain::readability), and never averaged with
//! it: half of an analysis can be *correct* and half can only be *plausible*,
//! and one blended number would hide which half a reader should trust.
//!
//! # The score is arithmetic, not an opinion
//!
//! The model is asked for verdicts and evidence, **not for a score**. The score
//! is computed here from the verdicts it gave. That is deliberate: a model
//! asked for both can return a number that does not follow from its own rows,
//! and a reader who disagrees with one row would have no way to see what it did
//! to the total. Computing it means the number is always reconstructible from
//! what is shown.

use serde::{Deserialize, Serialize};

/// How well one stated requirement is answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// The CV evidences it.
    Met,
    /// The CV gestures at it without evidencing it.
    Partial,
    /// The CV does not address it.
    Missing,
}

impl Verdict {
    /// What this verdict contributes to the score.
    ///
    /// `Partial` is half rather than some tuned figure because there is nothing
    /// to tune it against — no employer's system is being consulted, and a
    /// precise-looking weight would imply a precision this does not have.
    fn weight(&self) -> f32 {
        match self {
            Verdict::Met => 1.0,
            Verdict::Partial => 0.5,
            Verdict::Missing => 0.0,
        }
    }
}

/// One requirement from the posting, and how the CV answers it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RequirementMatch {
    /// The requirement, in the posting's own words.
    pub text: String,

    /// How well the CV answers it.
    pub verdict: Verdict,

    /// The CV line this was judged against.
    ///
    /// **This is what makes the estimate auditable rather than oracular.** A
    /// reader can check the quote, disagree with one row, and still trust the
    /// rest — which is impossible when a score arrives with no working shown.
    /// `None` for a requirement nothing in the CV addresses.
    pub evidence: Option<String>,
}

/// The estimated half of an analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RelevanceReport {
    /// Percentage, computed from the verdicts below and nothing else.
    pub score: u8,

    /// Every requirement the model found, in the posting's order.
    pub requirements: Vec<RequirementMatch>,
}

impl RelevanceReport {
    /// Builds a report, computing the score from the verdicts.
    ///
    /// The only constructor, so a report whose score disagrees with its rows
    /// cannot be made.
    pub fn from_requirements(requirements: Vec<RequirementMatch>) -> Self {
        // No requirements is not a perfect match. A posting nothing could be
        // extracted from tells us nothing about the CV, and reporting 100
        // would read as "you answer everything".
        let score = if requirements.is_empty() {
            0
        } else {
            let total: f32 = requirements.iter().map(|r| r.verdict.weight()).sum();
            ((total / requirements.len() as f32) * 100.0).round() as u8
        };

        Self {
            score,
            requirements,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(verdict: Verdict) -> RequirementMatch {
        RequirementMatch {
            text: "Kafka in production".into(),
            verdict,
            evidence: None,
        }
    }

    #[test]
    fn every_requirement_met_is_full_marks() {
        let report = RelevanceReport::from_requirements(vec![req(Verdict::Met); 3]);

        assert_eq!(report.score, 100);
    }

    #[test]
    fn nothing_met_is_zero() {
        let report = RelevanceReport::from_requirements(vec![req(Verdict::Missing); 3]);

        assert_eq!(report.score, 0);
    }

    #[test]
    fn a_partial_counts_for_half() {
        let report =
            RelevanceReport::from_requirements(vec![req(Verdict::Met), req(Verdict::Missing)]);
        assert_eq!(report.score, 50);

        let all_partial = RelevanceReport::from_requirements(vec![req(Verdict::Partial); 4]);
        assert_eq!(all_partial.score, 50);
    }

    /// The property the frontend asked for: the score follows from the rows,
    /// so a reader who disagrees with one can see what it did to the total.
    #[test]
    fn the_score_is_always_reconstructible_from_the_rows() {
        let requirements = vec![
            req(Verdict::Met),
            req(Verdict::Partial),
            req(Verdict::Missing),
            req(Verdict::Met),
        ];
        let report = RelevanceReport::from_requirements(requirements.clone());

        let expected: f32 = requirements
            .iter()
            .map(|r| match r.verdict {
                Verdict::Met => 1.0,
                Verdict::Partial => 0.5,
                Verdict::Missing => 0.0,
            })
            .sum::<f32>()
            / requirements.len() as f32
            * 100.0;

        assert_eq!(report.score, expected.round() as u8);
    }

    /// A posting nothing could be read out of tells us nothing about the CV.
    /// Reporting 100 would say "you answer everything".
    #[test]
    fn no_requirements_is_zero_not_a_perfect_match() {
        let report = RelevanceReport::from_requirements(vec![]);

        assert_eq!(report.score, 0);
        assert!(report.requirements.is_empty());
    }

    /// The wire form is what a client branches on.
    #[test]
    fn verdicts_serialise_as_the_frontend_expects() {
        for (verdict, wire) in [
            (Verdict::Met, "\"met\""),
            (Verdict::Partial, "\"partial\""),
            (Verdict::Missing, "\"missing\""),
        ] {
            assert_eq!(serde_json::to_string(&verdict).unwrap(), wire);
        }
    }
}
