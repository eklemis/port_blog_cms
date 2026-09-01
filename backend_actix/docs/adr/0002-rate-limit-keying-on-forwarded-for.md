# 0002 — Rate-limit callers are keyed on `X-Forwarded-For`

**Status:** Accepted
**Component:** `shared/rate_limit::policy`

## Context

The rate limiter needs a per-caller key. The obvious choice is the peer address
of the TCP connection.

Behind Cloud Run that address is the load balancer for *every* request.

## Decision

**Callers are keyed on the left-most `X-Forwarded-For` entry, falling back to
the peer address when the header is absent.**

## Consequences

Keying on the peer address would collapse every client onto one counter and let
one busy caller lock out everybody. The header is the only thing that
distinguishes callers in this deployment.

**This is only sound behind a proxy that overwrites the header.** Cloud Run
does. A proxy that merely *appends* would let a caller send their own
`X-Forwarded-For` and rotate the value for a fresh bucket per request, which
removes the limit entirely for anyone who notices.

That makes the deployment topology load-bearing. **If this service is ever put
behind a different proxy, or exposed directly, this decision has to be revisited
before the change ships** — nothing in the code will fail, the limiter will just
quietly stop limiting.

The fallback to peer address matters for local development, where there is no
proxy and no header.

## Alternatives considered

**Key on the authenticated user.** Not possible: these are the *un*authenticated
endpoints, which is the whole reason they need limiting.

**Trust a configured number of proxy hops.** More correct in general, and worth
doing if the topology ever stops being "exactly one proxy that overwrites". It
is more configuration than the current single deployment justifies.
