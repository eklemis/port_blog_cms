//! Reading and spending a generation allowance.

use async_trait::async_trait;
use chrono::Utc;

use crate::ai::application::ports::incoming::use_cases::{
    ConsumeAiQuotaUseCase, GetAiQuotaUseCase, QuotaError,
};
use crate::ai::application::ports::outgoing::UsageCounter;
use crate::ai::domain::quota::{period_end, period_key, QuotaState};
use crate::auth::application::domain::entities::UserId;

/// How many generations a person gets per calendar month.
///
/// Read from `AI_MONTHLY_QUOTA`. **Unset means unmetered**, and unmetered still
/// counts — which is the whole reason this exists before any limit has been
/// chosen. When the time comes to pick a number, it can be picked from what
/// people actually did rather than from a guess, and the screens that display
/// it are already built.
#[derive(Debug, Clone, Copy, Default)]
pub struct QuotaPolicy {
    /// The ceiling, or `None` for unmetered.
    pub monthly_limit: Option<u32>,
}

impl QuotaPolicy {
    /// Reads the policy from the environment.
    ///
    /// A malformed value is treated as unmetered rather than as zero, and says
    /// so in the log: a typo in configuration must not silently refuse every
    /// generation in the product.
    pub fn from_env() -> Self {
        let monthly_limit = match std::env::var("AI_MONTHLY_QUOTA") {
            Err(_) => None,
            Ok(raw) => match raw.trim().parse::<u32>() {
                Ok(n) => Some(n),
                Err(_) => {
                    tracing::warn!(
                        "AI_MONTHLY_QUOTA is set to {raw:?}, which is not a number. \
                         Treating generation as unmetered."
                    );
                    None
                }
            },
        };

        Self { monthly_limit }
    }
}

/// Implements both quota contracts over one counter.
pub struct QuotaService<C> {
    counter: C,
    policy: QuotaPolicy,
}

impl<C> QuotaService<C> {
    /// Builds it from the ports it depends on.
    pub fn new(counter: C, policy: QuotaPolicy) -> Self {
        Self { counter, policy }
    }
}

#[async_trait]
impl<C> GetAiQuotaUseCase for QuotaService<C>
where
    C: UsageCounter + Send + Sync,
{
    async fn execute(&self, owner: UserId) -> Result<QuotaState, QuotaError> {
        let now = Utc::now();
        let used = self.counter.used(owner.value(), &period_key(now)).await?;

        Ok(QuotaState {
            used,
            limit: self.policy.monthly_limit,
            resets_at: period_end(now),
        })
    }
}

#[async_trait]
impl<C> ConsumeAiQuotaUseCase for QuotaService<C>
where
    C: UsageCounter + Send + Sync,
{
    async fn execute(&self, owner: UserId) -> Result<QuotaState, QuotaError> {
        let now = Utc::now();
        let period = period_key(now);
        let resets_at = period_end(now);

        // Checked before recording, so a refused call does not spend anything.
        // The read-then-write race can over-admit by one under concurrency,
        // which is the right way round: two simultaneous requests at the
        // boundary both succeeding is a rounding error, and both failing when
        // one had budget is a bug someone reports.
        let used = self.counter.used(owner.value(), &period).await?;

        let state = QuotaState {
            used,
            limit: self.policy.monthly_limit,
            resets_at,
        };

        if state.is_exhausted() {
            return Err(QuotaError::Exceeded(Box::new(state)));
        }

        let ttl = (resets_at - now).num_seconds().max(1) as u64;
        let used = self.counter.record(owner.value(), &period, ttl).await?;

        Ok(QuotaState {
            used,
            limit: self.policy.monthly_limit,
            resets_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::application::ports::outgoing::UsageCounterError;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    #[derive(Default)]
    struct FakeCounter {
        count: Mutex<u32>,
        unavailable: bool,
        ttls: Mutex<Vec<u64>>,
    }

    #[async_trait]
    impl UsageCounter for Arc<FakeCounter> {
        async fn used(&self, _o: Uuid, _p: &str) -> Result<u32, UsageCounterError> {
            if self.unavailable {
                return Err(UsageCounterError::Unavailable("redis down".into()));
            }
            Ok(*self.count.lock().unwrap())
        }

        async fn record(
            &self,
            _o: Uuid,
            _p: &str,
            expires_in_secs: u64,
        ) -> Result<u32, UsageCounterError> {
            if self.unavailable {
                return Err(UsageCounterError::Unavailable("redis down".into()));
            }
            self.ttls.lock().unwrap().push(expires_in_secs);
            let mut c = self.count.lock().unwrap();
            *c += 1;
            Ok(*c)
        }
    }

    fn service(counter: Arc<FakeCounter>, limit: Option<u32>) -> QuotaService<Arc<FakeCounter>> {
        QuotaService::new(
            counter,
            QuotaPolicy {
                monthly_limit: limit,
            },
        )
    }

    fn owner() -> UserId {
        UserId::from(Uuid::new_v4())
    }

    // ── unmetered ──────────────────────────────────────────────────────

    /// The point of shipping this before a limit exists: usage is counted from
    /// day one, so the eventual number is chosen from real data.
    #[tokio::test]
    async fn an_unmetered_quota_still_counts() {
        let counter = Arc::new(FakeCounter::default());
        let svc = service(Arc::clone(&counter), None);

        ConsumeAiQuotaUseCase::execute(&svc, owner()).await.unwrap();
        let state = ConsumeAiQuotaUseCase::execute(&svc, owner()).await.unwrap();

        assert_eq!(state.used, 2);
        assert_eq!(state.limit, None);
    }

    #[tokio::test]
    async fn an_unmetered_quota_never_refuses() {
        let counter = Arc::new(FakeCounter::default());
        *counter.count.lock().unwrap() = 100_000;

        let result = ConsumeAiQuotaUseCase::execute(&service(counter, None), owner()).await;

        assert!(result.is_ok());
    }

    // ── metered ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn a_metered_quota_refuses_once_it_is_spent() {
        let counter = Arc::new(FakeCounter::default());
        *counter.count.lock().unwrap() = 5;

        let err = ConsumeAiQuotaUseCase::execute(&service(Arc::clone(&counter), Some(5)), owner())
            .await
            .unwrap_err();

        match err {
            QuotaError::Exceeded(state) => {
                assert_eq!(state.used, 5);
                assert_eq!(state.limit, Some(5));
            }
            other => panic!("expected Exceeded, got {other:?}"),
        }
    }

    /// A refused call must not spend anything, or a client retrying would dig
    /// itself deeper for no work done.
    #[tokio::test]
    async fn a_refused_call_records_nothing() {
        let counter = Arc::new(FakeCounter::default());
        *counter.count.lock().unwrap() = 5;

        let _ =
            ConsumeAiQuotaUseCase::execute(&service(Arc::clone(&counter), Some(5)), owner()).await;

        assert_eq!(*counter.count.lock().unwrap(), 5);
    }

    /// Reading where you stand has to work when you have nothing left — that
    /// is exactly when someone looks.
    #[tokio::test]
    async fn reading_the_quota_works_when_it_is_exhausted() {
        let counter = Arc::new(FakeCounter::default());
        *counter.count.lock().unwrap() = 5;

        let state = GetAiQuotaUseCase::execute(&service(counter, Some(5)), owner())
            .await
            .unwrap();

        assert_eq!(state.remaining(), Some(0));
        assert!(state.is_exhausted());
    }

    #[tokio::test]
    async fn reading_the_quota_spends_nothing() {
        let counter = Arc::new(FakeCounter::default());

        GetAiQuotaUseCase::execute(&service(Arc::clone(&counter), Some(5)), owner())
            .await
            .unwrap();

        assert_eq!(*counter.count.lock().unwrap(), 0);
    }

    /// The counter expires with its period, so a key does not accumulate per
    /// user per month forever.
    #[tokio::test]
    async fn the_counter_is_given_the_rest_of_the_period_to_live() {
        let counter = Arc::new(FakeCounter::default());

        ConsumeAiQuotaUseCase::execute(&service(Arc::clone(&counter), Some(5)), owner())
            .await
            .unwrap();

        let ttl = counter.ttls.lock().unwrap()[0];
        assert!(ttl > 0, "a zero TTL would delete the counter immediately");
        assert!(ttl <= 31 * 24 * 3600, "no period is longer than a month");
    }

    // ── the store being down ───────────────────────────────────────────

    /// Unlike the rate limiter, this fails **closed**. The limiter protects
    /// against abuse and refusing everyone during an outage would be worse
    /// than the abuse; a quota protects against cost, and admitting unbounded
    /// paid generations because a cache is down is the failure that shows up
    /// on an invoice.
    #[tokio::test]
    async fn an_unreachable_counter_refuses_rather_than_admitting() {
        let counter = Arc::new(FakeCounter {
            unavailable: true,
            ..Default::default()
        });

        let err = ConsumeAiQuotaUseCase::execute(&service(counter, Some(5)), owner())
            .await
            .unwrap_err();

        assert!(matches!(err, QuotaError::Unavailable(_)));
    }
}
