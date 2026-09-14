# Validation rules

What the server rejects, and with which error code. Every rule here is enforced
in the service layer — reaching the endpoint with bad input produces a `400` and
one of the codes in [`API_ERRORS.md`](API_ERRORS.md), never a partial write.

**Most of these are not in [`openapi.json`](openapi.json).** The spec declares a
handful of `maxLength` and `minLength` constraints, but the majority live in
Rust and are invisible to a generated client. That is the reason this file
exists: a form built only from the spec will let a user submit input the server
then refuses.

## Accounts

`POST /api/auth/register` — body: `username`, `full_name`, `email`, `password`,
all required.

| Field | Rule | Code on failure |
| --- | --- | --- |
| `username` | Trimmed, non-empty, 3–50 characters | `INVALID_USERNAME` |
| `username` | Letters, digits and `_` only — no spaces, dots or hyphens | `INVALID_USERNAME` |
| `full_name` | Trimmed, non-empty, at most 100 characters | `INVALID_FULL_NAME` |
| `email` | Must parse as an email address | `INVALID_EMAIL` |
| `password` | 12–128 characters | `INVALID_PASSWORD` |
| `email`, `username` | Must not already be taken | `409` `USER_ALREADY_EXISTS` |

Leading and trailing whitespace is trimmed before every check, so `"  ab  "` is
a 2-character username and fails.

`USER_ALREADY_EXISTS` is returned for a taken email **and** for a taken
username — it does not say which. If you want to tell the user which field to
change, check availability before submitting rather than parsing the message.

**The 128-character password ceiling is deliberate**, not an oversight. The
password is hashed with Argon2, and an unbounded input is unbounded work for the
server to do on an unauthenticated request.

The same 12–128 rule applies to `POST /api/auth/password-reset/{token}`, where
it *is* declared in the spec as `ResetPasswordDto.password`.

## Content

| Endpoint | Field | Rule | Code |
| --- | --- | --- | --- |
| `POST /api/blog` | `slug` | Trimmed, lowercased, non-empty, ≤ 200 chars | `INVALID_SLUG` |
| `POST /api/blog` | `slug` | **`a-z`, `0-9` and `-` only** | `INVALID_SLUG` |
| `POST /api/blog` | `title` | Non-empty, ≤ 200 characters | `INVALID_TITLE` |
| `PATCH /api/blog/{id}` | `content` | **Non-empty when publishing** — a draft may be blank | `INVALID_CONTENT` |
| `POST /api/topics` | `title` | 1–100 characters | `INVALID_TITLE` |
| `PATCH /api/topics/{id}` | `title` | 1–100 characters | `INVALID_TITLE` |

**Slugs are lowercased on write.** The uniqueness index is on `lower(slug)`, so
`My-Post` and `my-post` collide; a duplicate is `409`, not a silent overwrite.
Check availability first with `GET /api/blog/slug-available` or
`GET /api/projects/slug-available` if you want to warn before submitting.

**A slug is `[a-z0-9-]` and nothing else.** Spaces and punctuation are refused
rather than escaped, so `My Post Title` is an `INVALID_SLUG`, not a slug with
`%20` in it. The check runs on `POST /api/blog` and on `PATCH` alike.

Note the test is `is_ascii_alphanumeric`, so **accented and non-Latin letters are
refused too** — `café` and `日本語` are not storable as slugs even though they are
perfectly good titles. If you generate a slug from a title, transliterate it;
lowercasing and replacing spaces is not enough.

**`content` may be empty on create, but not on publish.** A post can be created
blank and written into afterwards; what it cannot do is go in front of a reader
that way — see below.

### Creating a post before writing it

`POST /api/blog` accepts an empty `content`, so a post can exist before it has
been written. Media attaches to a target id, so a cover image needs the post to
exist first, and requiring a body at creation meant writing prose before a
picture could be added.

**The key must still be present.** `content` is a required *field* — send
`"content": ""`, not nothing at all. Omitting it fails to deserialise, which is
a 400 about a malformed body rather than an empty post.

The guard against empty posts was not dropped, only moved. **Publishing a post
with an empty body is refused** — `PATCH /api/blog/{id}` setting `published_at`
returns `400` `INVALID_CONTENT` unless the post will have a body afterwards.
The body may arrive in the same request that publishes it.

What is checked is what the post *will* hold: the incoming `content` when the
request supplies it, the stored content when it does not, and empty when the
request clears it. So publishing an already-written post needs to send no
content at all, and `{"published_at": "…", "content": null}` is refused rather
than quietly publishing a blank page.

An unfinished draft sitting in its author's own console was never the thing the
rule protected against.

## Uploads

`POST /api/media/upload-url`. **The bytes never pass through the API** — the
client PUTs them straight to the bucket — so every rule here is checked against
what the client *claims* in the request body:

| Field | Rule | Code |
| --- | --- | --- |
| `file_size_bytes` | ≤ 5 MB (5 242 880) | `FILE_TOO_LARGE` |
| `mime_type` | One of `image/jpeg`, `image/png`, `image/webp` | `INVALID_MIME_TYPE` |
| `width_px`, `height_px` | Each ≤ 6000 | `INVALID_DIMENSIONS` |
| `original_filename` | Non-empty, ≤ 255 chars, no path separator | `INVALID_FILE_NAME` |
| extension | Must match the declared MIME type | `INVALID_EXTENSION` |

Because these are claims rather than measurements, **treat them as a usability
check, not a security boundary** — the bucket enforces its own limits, and a
client that lies gets a rejected upload rather than a rejected request.

`role` and `attachment_target` take the lowercase wire values
(`avatar`, `cover`, `screenshot`; `user`, `resume`, `blog_post`), the same
strings every response returns.

## Partial updates

`PATCH` bodies distinguish three states, and getting this wrong is the most
common way to lose data:

```json
{ }                      // field absent  → leave it alone
{ "alt_text": null }     // explicit null → clear it
{ "alt_text": "A hen" }  // value         → set it
```

Applies to `PATCH /api/media/{id}`, `PUT /api/users/me` (`bio`), and the blog and
cover-letter editors.

**If your form serialises untouched fields as `null`, a name-only edit will wipe
a bio.** Send only what changed.

Two fields are exceptions because they have no meaningful empty value:
`position` on media and `locale` on the account are plain optionals, where
`null` and absent mean the same thing.

## Bulk requests

`POST /api/blog/bulk`, `/api/projects/bulk`, `/api/media/bulk`:

| Rule | Code |
| --- | --- |
| At least one id | `BULK_EMPTY` |
| At most 100 ids | `BULK_TOO_LARGE` |

Those two are request-level and return `4xx`. Per-item failures do **not** —
see [`PAGINATION.md`](PAGINATION.md#bulk-is-not-paginated-and-not-atomic).

## State rules

Not input validation, but the same class of surprise — a request that looks
well formed and is refused:

| Rule | Code |
| --- | --- |
| An application cannot leave `draft` without a CV snapshot | `400` `SNAPSHOT_REQUIRED` |
| Generation refuses when the period's allowance is spent | `429` `AI_QUOTA_EXCEEDED` |
| Generation routes on a deployment with no provider configured | `503` `AI_DISABLED` |

`SNAPSHOT_REQUIRED` is also enforced by a database constraint, so it can arrive
from a race even when the request looked valid a moment earlier. Handle it as a
normal validation failure, not an exceptional one.
