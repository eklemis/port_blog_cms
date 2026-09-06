# 0010 — "Additive" is the wrong test; backward-compatible with the running build is

**Status:** Accepted
**Component:** `deploy.sh`, `migration/`
**Amends:** [0003](0003-migrate-before-deploy.md)

## Context

[ADR 0003](0003-migrate-before-deploy.md) fixed the deploy ordering: migrations
run before the Cloud Run service update. Its safety argument for the window
between the two was that migrations are **additive**, so the still-running old
build ignores what it does not know about. The rule it drew from that was
"migrations must stay additive", and the example it warned about was a
destructive one — dropping a column the old build still selects.

`m20260906_000001_sent_applications_require_a_snapshot` satisfied that rule and
broke the argument anyway. It adds a `CHECK` constraint:

```sql
CHECK (status = 'draft' OR cv_snapshot_id IS NOT NULL)
```

Nothing is dropped or renamed. In DDL terms it is purely additive. But a
constraint is not a table the old build can ignore — **the database enforces it
against whatever code is running**, and in the window before the service update
that is the old build. Any write the old build considers legal and the new
constraint does not now fails, and fails as a 500, because the code that maps
the violation to a domain error ships with the deploy that has not happened yet.

So "additive" was never the property that made the window safe. It was a proxy
for the property that does: that the old build and the new schema can coexist.
Adding a column preserves that. Adding a constraint does not, and neither does
adding a `NOT NULL`, a unique index over data the old build can still duplicate,
or a foreign key it can still violate.

## Decision

**A migration must be backward-compatible with the build that is still running
when it applies — not merely additive.**

The test to apply is not "does this drop anything?" but:

> If the old build kept serving traffic against this schema for the next few
> minutes, is every write it might make still legal?

When the answer is no, the change splits across two deploys, in the order that
keeps every intermediate state legal:

| Change | First deploy | Second deploy |
| --- | --- | --- |
| Drop a column | Stop selecting it | Drop it |
| Add a constraint | Stop writing rows that violate it | Add the constraint |
| Add `NOT NULL` | Start writing the column always, backfill | Add the constraint |

Note that the two rows are mirror images: for a removal the code goes first
because the schema is what breaks; for a constraint the code goes first because
the schema is what enforces. In both, **the deploy that narrows what is legal
goes second.**

## Consequences

The constraint migration was survivable only by accident of ordering. The
service-level check it backstops — `ApplicationService` refusing to leave
`draft` without a snapshot — had already shipped weeks earlier with the career
tables. By the time the constraint applied, the running build had long since
stopped producing rows that violate it, which is exactly the two-deploy split
this record describes, arrived at without anyone deciding to.

That is not a reason to relax the rule. It is the reason to write it down: the
migration passed the stated test, violated the real one, and was safe anyway for
a reason nobody checked at the time.

`m20260906_000001` also carries a data repair — it returns already-sent
applications with no snapshot to `draft` — which the "additive" framing did not
contemplate either. A migration that rewrites rows is a migration that can be
wrong about live data, and it deserves the same pre-flight as any other
destructive change: count the affected rows first, and know the number before
applying.

ADR 0003's ordering decision stands unchanged. Only its precondition is
restated here, and its wording is left as it was written, per the immutability
note in [the index](README.md).

## Alternatives considered

**Add constraints as `NOT VALID`.** Rejected: it skips validating *existing*
rows, but still enforces every new write — which is the half that breaks the old
build. It solves a different problem (a long validation lock on a large table),
not this one.

**Have `deploy.sh` detect incompatible migrations.** Rejected: the property is
semantic, not syntactic. Whether a constraint is compatible depends on what the
running build writes, which the script cannot inspect. A check that catches
`ALTER TABLE ... ADD CONSTRAINT` and nothing else would be a rule people learn
to route around rather than a safeguard.
