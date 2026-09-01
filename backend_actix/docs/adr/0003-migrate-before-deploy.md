# 0003 — Migrations run before the Cloud Run update, not on startup

**Status:** Accepted
**Component:** `deploy.sh`, `migration/`

## Context

The schema is owned by the `migration` crate beside the API. Something has to
apply it, and there are three moments available: on container startup, after the
deploy, or before it.

## Decision

**`deploy.sh` applies pending migrations before updating the Cloud Run service,
and a migration failure aborts the deploy.**

The container does not migrate on startup.

## Consequences

Shipping code ahead of its schema would leave the new routes failing at request
time with `relation ... does not exist` while the deploy itself reported
success. Applying first avoids that, and is safe in the other direction:
migrations here are additive, so the still-running old build simply ignores
tables it does not know about.

Two details that exist because of this ordering, and should not be undone:

- **The database URL is read back from Secret Manager rather than from the
  prompt**, so the migration always targets the same database Cloud Run will
  connect to — including on runs where the secret was left unchanged. It is
  never echoed.
- **A missing `cargo` aborts the script** rather than skipping the step. A quiet
  skip is exactly the failure this ordering exists to prevent.

`deploy.sh` shows `migration status` and asks for confirmation before applying.
`SKIP_MIGRATIONS=1` skips the step; `AUTO_MIGRATE=1` applies without the prompt,
for CI.

The requirement that migrations stay additive is now load-bearing. A destructive
migration — dropping a column the running build still selects — breaks
production between the migration and the service update. Such a change has to be
split across two deploys: stop using the column, then remove it.

## Alternatives considered

**Migrate on container startup.** Rejected: with more than one Cloud Run
instance, several containers race to migrate the same database on every scale-up,
and a migration failure becomes a crash loop rather than a failed deploy.

**Migrate after the service update.** Rejected: that is the window this decision
exists to close.
