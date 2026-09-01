# 0001 — The rate limiter fails open when Redis is unreachable

**Status:** Accepted
**Component:** `shared/rate_limit`

## Context

The unauthenticated auth endpoints are rate-limited per caller, with the
counters held in Redis:

| Endpoint | Limit |
| --- | --- |
| `POST /api/auth/login` | 10 / 5 min |
| `POST /api/auth/register` | 5 / hour |
| `POST /api/auth/password-reset` | 5 / hour |
| `POST /api/auth/password-reset/{token}` | 10 / hour |
| `POST /api/auth/refresh` | 30 / 5 min |

Authenticated routes are not limited: reaching them already requires a valid
token. The limits are low because each of these costs an Argon2 hash, and
registration also sends mail — they are a denial-of-service lever as much as a
credential-guessing one.

Redis is a separate managed service and can be unreachable while the API is
healthy. The middleware has to decide what to do with a request it cannot
count.

## Decision

**If Redis is unreachable the limiter allows the request and logs.**

## Consequences

Refusing every login during a cache outage would turn it into a total
authentication outage. The limiter is a mitigation, not the security
boundary — the credential check behind it is.

The cost is that during a Redis outage the protected endpoints are unlimited,
so an attacker who can cause or wait out an outage gets an unthrottled window
against Argon2. That is accepted: an attacker who can take down Redis has a
cheaper denial-of-service available already.

This is why `token_repository_redis` and `rate_limit::redis_store` have
integration tests that need a live Redis. The middleware tests use an in-memory
store that reimplements the counting, so they cannot catch a mistake in the
Redis commands themselves.

## Alternatives considered

**Fail closed.** Rejected: it converts a cache outage into an outage of the
whole product, which is strictly worse than the risk it removes.

**Fall back to an in-process counter.** Rejected: with more than one Cloud Run
instance the counters diverge, so the limit becomes "N × instances" — a number
nobody can reason about, and one that silently changes with autoscaling.
