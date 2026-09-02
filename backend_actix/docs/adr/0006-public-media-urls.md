# 0006 — Public media is served by a signing redirect, not a public bucket

**Status:** Accepted
**Component:** `multimedia` public read path, `blog` public responses

## Context

Media variants live in a GCS bucket and were reachable only through
`GET /api/media/{id}/{size}`, which sits behind the `VerifiedUser` extractor.
No public response carried a media field. So a published post's cover was
invisible to readers — a portfolio site that could not show the work. An
anonymous visitor holds no token, and the SvelteKit server has no credential to
borrow.

The obvious fix is to put signed URLs in the public response. That does not
work, and understanding why is what rules out most of the alternatives.

**A signed URL cannot survive caching.** Signed URLs carry a short expiry. A
server-rendered public page is cached; a page cached for an hour with a
fifteen-minute URL baked into it serves dead links for forty-five minutes. The
failure is invisible in development, where nothing is cached that long, and
appears in production as intermittently broken images.

## Decision

**The ready bucket stays private. Public responses carry a stable URL into this
API — `GET /api/public/media/{media_id}/{size}` — which checks visibility, signs
a short-lived URL, and returns `302`.**

The console keeps `GET /api/media/{id}/{size}` and its signed URLs.

The caching problem disappears because the string in the cached page is the API
path, which never expires. The signature is minted per fetch and only has to
live for the redirect hop.

## Consequences

**The bucket is never public.** No object is reachable except through this API,
which is the property the deployment requires. Nothing needs to be granted to
`allUsers`, and there is no deploy step attached to this change.

**Access is revocable, which a public bucket could never be.** The lookup joins
the variant to what it is attached to and requires a blog post that is
published, not scheduled, and not deleted; it also requires the media itself not
to be soft-deleted. Unpublish the post and the endpoint 404s from the next
request. With a world-readable bucket, a URL that escaped stayed valid forever.

**A missing variant and a non-visible one are the same 404**, deliberately.
Telling them apart would let a caller probe for media attached to drafts.

**The cost is one API request per image.** No bytes pass through the API — the
redirect sends the browser to GCS — but the request still costs a Cloud Run
invocation. The redirect carries `Cache-Control: public, max-age=300`, shorter
than the signed URL it points at, so a cached redirect never outlives its
target.

**The visibility rule lives in one SQL statement.** Adding projects or CVs to
the public surface means another arm in `find_public_variant_stmt`, not another
route. That is deliberate: a second place to decide "may a reader see this" is a
second place to get it wrong.

## Alternatives considered

**A world-readable ready bucket, with plain URLs in public responses.** This was
the first implementation and it was wrong. It solves caching but makes every
object permanently and unrevocably fetchable by anyone holding a URL, including
after the post referencing it is unpublished. It also contradicts the
deployment's premise that the bucket is reachable only through the backend.

An earlier draft of this record rejected the signing-redirect option on the
grounds that it "reintroduces the expiry problem for cached pages". **That was a
reasoning error.** The cached page holds the API path, not the signed URL, so
there is no expiry to reintroduce. The mistake is recorded here because the
argument sounded right and could easily be made again.

**Proxying the bytes through the API.** Same privacy and revocation properties,
and it never exposes the storage host. Rejected on cost: every image on every
page becomes Cloud Run bandwidth and request time, where the redirect costs a
few hundred bytes.

**Cloud CDN with an authenticated origin in front of the private bucket.** The
right answer at a larger scale — best performance, no per-image API hit — and
compatible with this decision, since it would sit in front of the same private
bucket. Rejected for now as real infrastructure work whose revocation story is
cache invalidation rather than an immediate 404.
