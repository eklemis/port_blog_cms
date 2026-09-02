# 0007 — Renaming the `screenshoot` media role takes three deploys

**Status:** Accepted — step 1 shipped, steps 2 and 3 pending
**Component:** `multimedia/`, `migration/`

## Context

`media_attachments.role` stores `"screenshoot"` for project screenshots. The
migration that created the table documents the value as `"screenshot"`, but the
comment was never executed — the code has always written the misspelling, so
every row in every environment says `"screenshoot"`.

The value is not internal. `PublicMedia.role` is a `String` built from
`Display`, so the misspelling is on the wire and clients read it today.

Correcting it is not a one-line change, because of the ordering fixed by
[ADR 0003](0003-migrate-before-deploy.md): migrations run **before** the service
is updated. A migration that renamed the rows would land while the previous
build was still serving, and that build parses only `"screenshoot"` — every
media read would start returning `invalid media role: screenshot` in the window
between the migration and the service update. The failure would be silent in
staging, where nothing is serving during a deploy, and loud in production.

## Decision

**Rename over three deploys, each safe on its own.**

1. **Read both, write the old one.** `parse_media_role` accepts `"screenshoot"`
   and `"screenshot"`; `Display` is untouched. No migration. Once this is
   deployed everywhere, every running build can read either spelling.
2. **Migrate the data.** `UPDATE media_attachments SET role = 'screenshot'
   WHERE role = 'screenshoot'`. Safe because of step 1.
3. **Write the new one.** `Display` emits `"screenshot"`. Parsing still accepts
   both, so a rollback to a step-2 build still reads every row.

A fourth change, later and optional, drops the legacy parse arm once no
environment can roll back past step 2.

**Step 3 is a breaking change for clients** that match on `role == "screenshoot"`
and must be announced before it ships, not after.

## Consequences

The obvious version of this change — one migration and one code edit — breaks
media reads during every deploy window. It looks correct in review and in a test
suite that never spans two builds, which is exactly why the sequence is written
down here rather than left to whoever picks it up.

The cost is that the misspelling stays readable in the code for three releases,
and a reader who sees only step 1 will think the dual parse is redundant. The
tests in `media_query_postgres.rs` name the step they belong to for that reason.

Steps must not be collapsed or reordered. Skipping step 1 is the outage
described above; skipping step 2 leaves step 3 writing a value that does not
match the rows already stored.

## Related

A separate defect found while mapping this one, and **not** addressed here:
`MediaRole` and `AttachmentTarget` derive `Serialize`/`Deserialize` without
`#[serde(rename_all)]`, unlike `MediaState` and `MediaSize`. Inbound JSON
therefore uses the Rust variant names (`"Screenshoot"`, `"BlogPost"`) while
`Display` — and so the database and every public response — uses lowercase
(`"screenshoot"`, `"blog_post"`). A client cannot feed a role it read from a
public response back into `POST /api/media/init-upload`. Fixing that is also a
client-visible contract change and needs its own decision.
