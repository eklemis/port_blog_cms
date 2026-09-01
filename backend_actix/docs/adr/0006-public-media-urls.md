# 0006 — Public responses carry unsigned media URLs

**Status:** Accepted
**Component:** `multimedia` storage port, `blog` public read path

## Context

Media variants live in a GCS bucket and were reachable only two ways: through
`GET /api/media/{id}/{size}`, which is behind the `VerifiedUser` extractor, or
not at all. No public response carried a media field.

So a published post's cover and a project's screenshots were invisible to
readers — a portfolio site that could not show the work. The frontend has no
way around it: an anonymous visitor holds no token, and the SvelteKit server has
no credential to borrow.

The obvious fix — let the public call the existing variant route — does not
work, and the reason is the decision here.

## Decision

**Objects in the ready bucket are publicly readable, and public responses
return plain, unsigned URLs. The console keeps signed URLs.**

`StorageQuery` gained `public_read_url`, alongside `get_signed_read_url`. The
two paths share one storage client.

## Consequences

**A signed URL cannot survive caching, and public pages are cached.** Signed
URLs carry a short expiry. A server-rendered page cached for an hour with a
fifteen-minute URL baked into it serves dead links for forty-five minutes. The
failure is invisible in development, where nothing is cached long enough, and
shows up as intermittently broken images in production. Lengthening the expiry
does not fix it; it only moves the boundary and weakens the signature's point.

**The cost is that a public object URL is permanent and unrevocable.** Anyone
who has the URL can fetch the object indefinitely — including after the post
referencing it is unpublished or deleted. Object keys carry a UUID so they are
not guessable, but "not guessable" is not "revocable".

That is accepted because the objects in question are *published work on a
portfolio site*. They are meant to be looked at. The alternative buys a weaker
version of the same exposure in exchange for a real caching bug.

**Two paths now exist for the same object, and they must not be crossed.** The
owner-facing `GET /api/blog/{post_id}` deliberately returns no media, so an
authenticated read cannot leak an unsigned URL into a context that assumed a
signed one. The public read is the only producer of unsigned URLs.

**This requires a bucket configuration change, not just code.** The ready
bucket must grant `roles/storage.objectViewer` to `allUsers`. Until that is
done the URLs are well-formed and return 403. The upload bucket must **not**
be made public — only the ready bucket, which holds generated variants.

## Alternatives considered

**A public variant route that signs on demand.** Rejected: it reintroduces the
expiry problem for cached pages, and adds an unauthenticated endpoint that
performs a signing operation per request.

**Long-expiry signed URLs — days rather than minutes.** Rejected: it trades a
caching bug for a leak with extra steps. A URL valid for a week is
substantially "public" already, but with machinery that implies otherwise.

**A CDN or proxy in front of the bucket.** The right answer at a larger scale,
and compatible with this decision — it would sit in front of the same public
objects. Not worth the infrastructure today.
