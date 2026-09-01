# Architecture decision records

Short records of decisions that were hard to make and would otherwise be
re-litigated, or quietly undone by someone who did not know why they were made.

0001-0004 were backfilled. The reasoning already existed — in `readme.md`, in
`build_steps.md`, in a CI comment — but it was scattered where nobody would
look for it. Each record here is a **move**, not a rewrite: the wording is the
original author's, and the source now points here.

An ADR is worth writing when the decision has a live alternative that a
reasonable person would pick, and choosing it would break something
non-obvious. Routine choices do not need one.

| # | Decision | Status |
| --- | --- | --- |
| [0001](0001-rate-limiter-fails-open.md) | The rate limiter fails open when Redis is unreachable | Accepted |
| [0002](0002-rate-limit-keying-on-forwarded-for.md) | Rate-limit callers are keyed on `X-Forwarded-For` | Accepted |
| [0003](0003-migrate-before-deploy.md) | Migrations run before the Cloud Run update, not on startup | Accepted |
| [0004](0004-llvm-cov-over-tarpaulin.md) | Coverage is measured with `cargo llvm-cov`, not tarpaulin | Accepted |
| [0005](0005-break-the-auth-email-cycle.md) | `email` does not depend on `auth` | Accepted |

## Format

Context, Decision, Consequences. No template ceremony beyond that. A record
that needs more than a page is usually two decisions.

Records are immutable once accepted. If a decision is reversed, add a new
record that supersedes it and update the status here, rather than editing
history — the point is to explain why the code looks the way it does, including
to someone reading an older commit.
