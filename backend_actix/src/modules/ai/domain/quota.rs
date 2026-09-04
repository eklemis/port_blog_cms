//! How much generation a person has used, and how much they may.

use chrono::{DateTime, Datelike, TimeZone, Utc};
use serde::Serialize;

/// A person's standing for the current period.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct QuotaState {
    /// Generations used in the current period.
    ///
    /// Counted whether or not a limit is configured. That is the point of
    /// having this endpoint before having a limit: the number that a sensible
    /// ceiling gets chosen from is real usage, not a guess.
    pub used: u32,

    /// The ceiling, or `null` when generation is currently unmetered.
    ///
    /// **`null` means "no limit configured", not "no limit possible".** A
    /// client should render the remaining count when a limit exists and stay
    /// quiet when it does not — but the surface that would show it should be
    /// built either way, because adding a limit later to screens that assume
    /// calls are free is the expensive order to do this in.
    pub limit: Option<u32>,

    /// When `used` returns to zero.
    pub resets_at: DateTime<Utc>,
}

impl QuotaState {
    /// Generations still available, or `None` when unmetered.
    pub fn remaining(&self) -> Option<u32> {
        self.limit.map(|limit| limit.saturating_sub(self.used))
    }

    /// Whether another generation would be refused.
    ///
    /// False when unmetered — an absent limit is not a limit of zero, and
    /// getting that backwards would refuse everything the moment the
    /// configuration was cleared.
    pub fn is_exhausted(&self) -> bool {
        self.remaining().map(|left| left == 0).unwrap_or(false)
    }
}

/// The key a period's counter lives under, and when it expires.
///
/// Calendar months rather than a rolling 30 days: a person can answer "when
/// does this reset" without being told, and the counter is trivially
/// inspectable. A rolling window is fairer and nobody can predict it.
pub fn period_key(now: DateTime<Utc>) -> String {
    format!("{:04}-{:02}", now.year(), now.month())
}

/// The first instant of the next calendar month.
pub fn period_end(now: DateTime<Utc>) -> DateTime<Utc> {
    let (year, month) = if now.month() == 12 {
        (now.year() + 1, 1)
    } else {
        (now.year(), now.month() + 1)
    };

    Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0)
        .single()
        .unwrap_or(now)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 12, 0, 0).unwrap()
    }

    #[test]
    fn a_period_is_a_calendar_month() {
        assert_eq!(period_key(at(2026, 9, 4)), "2026-09");
        assert_eq!(period_key(at(2026, 9, 30)), "2026-09");
        assert_ne!(period_key(at(2026, 9, 30)), period_key(at(2026, 10, 1)));
    }

    #[test]
    fn a_period_ends_at_the_start_of_the_next_month() {
        assert_eq!(
            period_end(at(2026, 9, 4)),
            at(2026, 10, 1) - chrono::Duration::hours(12)
        );
    }

    /// December is the one that gets rolled over wrong.
    #[test]
    fn december_rolls_into_the_next_year() {
        let end = period_end(at(2026, 12, 15));

        assert_eq!(end.year(), 2027);
        assert_eq!(end.month(), 1);
    }

    /// An absent limit is not a limit of zero. Getting this backwards would
    /// refuse every generation the moment the configuration was cleared.
    #[test]
    fn an_unmetered_quota_is_never_exhausted() {
        let state = QuotaState {
            used: 9_999,
            limit: None,
            resets_at: at(2026, 10, 1),
        };

        assert!(!state.is_exhausted());
        assert_eq!(state.remaining(), None);
    }

    #[test]
    fn a_metered_quota_reports_what_is_left() {
        let state = QuotaState {
            used: 40,
            limit: Some(100),
            resets_at: at(2026, 10, 1),
        };

        assert_eq!(state.remaining(), Some(60));
        assert!(!state.is_exhausted());
    }

    #[test]
    fn a_spent_quota_is_exhausted() {
        let state = QuotaState {
            used: 100,
            limit: Some(100),
            resets_at: at(2026, 10, 1),
        };

        assert!(state.is_exhausted());
        assert_eq!(state.remaining(), Some(0));
    }

    /// Usage can exceed the limit if one is lowered mid-period. That must read
    /// as "none left", not underflow into a huge number.
    #[test]
    fn lowering_the_limit_below_current_usage_does_not_underflow() {
        let state = QuotaState {
            used: 120,
            limit: Some(100),
            resets_at: at(2026, 10, 1),
        };

        assert_eq!(state.remaining(), Some(0));
        assert!(state.is_exhausted());
    }
}
