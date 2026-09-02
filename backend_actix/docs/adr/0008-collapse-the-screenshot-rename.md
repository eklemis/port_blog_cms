# 0008 — The `screenshot` rename shipped in one step, and the role wire format was fixed with it

**Status:** Accepted — supersedes the sequencing in [0007](0007-screenshot-role-rename.md)
**Component:** `multimedia/`, `migration/`

## Context

[ADR 0007](0007-screenshot-role-rename.md) split the `screenshoot` → `screenshot`
rename across three deploys. The hazard it protects against is real: under
[ADR 0003](0003-migrate-before-deploy.md) the migration lands before the service
is updated, so a rename would rewrite rows while the previous build — which
parses only the old spelling — was still answering requests.

That hazard needs a previous build. When the remaining steps came to be done,
the backend had not been rolled out anywhere. There was no serving revision to
break, and no deploy window to protect.

0007 also recorded a second defect found while mapping the first. `MediaRole`
and `AttachmentTarget` derived `Serialize`/`Deserialize` without
`#[serde(rename_all)]`, which their siblings `MediaState` and `MediaSize` both
carry. Inbound JSON therefore used Rust variant names (`"Screenshoot"`,
`"BlogPost"`) while `Display` — and so every stored row and every public
response — used lowercase. A client could not post back a role it had just
read. Fixing that is a client-visible contract change, which is expensive
against a deployed API and free against one that has never been deployed.

## Decision

**Both changes ship together, in one step.**

- The data migration renames the rows, and `Display` writes `screenshot`.
  Parsing still accepts `screenshoot`, which costs one match arm and means a
  database restored from a pre-rename backup still loads.
- `MediaRole` gains `#[serde(rename_all = "lowercase")]` and
  `AttachmentTarget` gains `#[serde(rename_all = "snake_case")]`, so the JSON
  form and the stored form are the same string for every variant.

`snake_case` rather than `lowercase` on `AttachmentTarget` because `BlogPost`
has always stored as `blog_post`; `lowercase` would have produced `blogpost`
and traded one mismatch for another.

The invariant is held by a test that walks every variant and asserts serde and
`Display` agree, rather than by asserting one spelling and trusting the other
to follow. That test is the part worth keeping — it is what would have caught
the original mismatch.

## Consequences

`POST /api/media/upload-url` now takes `"screenshot"` and `"blog_post"` where
it took `"Screenshoot"` and `"BlogPost"`. **Any client sending the capitalized
forms breaks**, and the frontend team needs telling — this is the whole reason
the change was cheap to make now and would not have been in six months.

0007's sequence was not wrong, and is not deleted. It is the correct procedure
for renaming a persisted value while something is serving, which is the
situation the next such rename will be in. What changed is the precondition,
not the reasoning. Anyone reaching for a single-step rename after this project
has a deployed backend should read 0007 first and assume it applies.

The dual parse arm is the one piece of 0007's caution that survives, because it
is nearly free and the failure it prevents — a restored backup erroring on
every project image — does not require a deploy window to happen.
