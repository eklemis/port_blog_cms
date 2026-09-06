# 0011 — A write is atomic by default, and the exceptions are deliberate

**Status:** Accepted
**Component:** every `*_postgres.rs` adapter, `shared/api/bulk.rs`, `ai/`, `multimedia/`

## Context

An audit of all 96 routes for atomicity and isolation found that most were
already correct, and not by accident. Writes are single statements — Postgres
makes those atomic on their own — uniqueness is enforced by database indexes
with SQLSTATE 23505 mapped to domain errors rather than by check-then-insert,
and two repositories already opened transactions where they genuinely wrote
several rows.

Eight places were not. They shared a shape: a sequence that reads as one
operation in Rust and reaches the database as several round trips.

- `media_repository_postgres::hard_delete` issued three deletes — variants,
  attachments, row — with nothing tying them together. Failing partway left the
  media row alive with its variants gone, which renders as an image that never
  loads and is **indistinguishable from one still being processed**. Clients are
  told to poll out of that state, so they would wait on a row that could never
  gain a variant again.
- Six `patch` / `archive` methods read a row and wrote it back on separate round
  trips with no lock, so two concurrent writers both read the same row and the
  second silently overwrote the first.
- The application transition read the row, decided the snapshot rule from that
  read, wrote a snapshot, then patched — two independent reads, no lock, so two
  concurrent sends could both find the rule satisfied.

The last one is the instructive case. Its orphaned snapshot turned out **not** to
be corruption, because a standalone snapshot is already a supported state — a CV
can be snapshotted directly. The defect was the stale read, not the missing
transaction, and a transaction alone would have been the wrong fix.

## Decision

**A write is atomic by default.** Concretely, four rules:

1. **More than one statement changing data is one transaction.** If a partial
   application of the sequence is a state the system should never be in, the
   statements commit together or not at all.
2. **A read whose value decides a later write happens in that same transaction,
   under `lock_exclusive()`.** Deciding on a row read a round trip earlier is
   deciding on a row that may already have changed.
3. **An invariant worth enforcing is a database constraint**, not only a service
   check. The service keeps the readable error — it is the one that can explain
   itself — and the adapter maps the violation back to that same error **by
   constraint name**, so a future constraint on the table is not mistaken for
   this one. See `applications_sent_requires_snapshot` and `db_err` in
   `application_store_postgres.rs`.
4. **Uniqueness is a unique index plus a 23505 mapping, never check-then-insert.**
   The check-then-insert version has a race that only appears under load, which
   is when it matters.

**And two things are exempt, deliberately.**

### Bulk endpoints are partial-success by contract

`shared/api/bulk.rs` states it: a bulk call *"is not one operation on a set — it
is many operations that each succeed or fail on their own"*. `success: true`
means the batch was processed, not that every item succeeded.

Making these transactional would be a plausible reading of "make writes atomic"
and it would be wrong. Archiving 99 posts should not fail because the 100th was
already deleted, and the frontend builds against the `failed[]` array. This is a
contract, not an oversight.

### Nothing spanning two systems can be a transaction

The AI allowance lives in Redis and the generation is an HTTP call to a vendor;
a media row is written now and the client uploads the bytes to object storage
later. No database transaction spans either pair, so the tools are different
ones — **idempotency, state machines, and reconciliation**:

- The allowance is spent *before* the provider is called, so an exhausted quota
  refuses without cost. A generation that then fails still counts, because the
  provider did the work.
- Media has an explicit `Pending → Processing → Ready/Failed` state machine, and
  a scheduled sweep removes registrations whose bytes never arrived.
- Redis counters repair their own missing expiry rather than assuming the write
  that created them also set one.

Reaching for a transaction here is not conservative, it is a category error.

## Consequences

The rules are cheap where they apply — a `begin()`, a `lock_exclusive()`, a
`CHECK` — and the audit that produced them took longer than the fixes did. That
is the expected ratio: the work is deciding *which* sequences are one operation,
not writing the transaction.

Rule 3 has a second-order benefit worth naming. Once the invariant is in the
schema, the service check stops being the only thing standing between the
product and a bad row, which means a future code path that forgets it produces a
clean domain error instead of silent corruption. That is what
`ApplicationStoreError::SnapshotRequired` exists for, and it is reachable only
by a race — which is precisely why it needs to exist.

The exemptions are the part most likely to be undone by someone applying rule 1
uniformly. Both are load-bearing, and both would look like bugs to a reader who
had only the rule.

## Alternatives considered

**Thread a transaction handle through the outgoing ports**, so a service could
run several port calls in one transaction. Rejected: it puts a persistence
detail in every port signature, including the ports that have nothing to do with
Postgres, to serve a handful of call sites. Where a service genuinely needs two
writes to commit together, the composite operation belongs behind one port
method whose adapter owns the transaction — or, as with the snapshot rule, the
invariant belongs in the schema instead.

**Make bulk endpoints all-or-nothing.** Rejected above, and rejected explicitly
rather than by omission, because it is the reading someone will arrive at.

**Serializable isolation globally.** Rejected: it converts these races into
serialization failures that every caller must now retry, which is a larger
change to every write path than the targeted locks, and it does nothing for the
two cross-system cases that motivated half this record.
