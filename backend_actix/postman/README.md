# Postman

`CV_API_Test.postman_collection.json` is an **end-to-end test suite**, not an
API reference: 132 requests carrying 124 test scripts and ~889 assertions,
organised as ordered scenarios that register users, verify them, exercise an
area, then clean up after themselves.

Despite the filename, the collection is called *Portfolio & Blog CMS* and is no
longer CV-only.

Run in order — the folders are a single scenario, not nine independent suites.
They share state through collection variables, so a folder run on its own will
fail on missing fixtures.

| # | Folder | Requests |
| --- | --- | --- |
| 1 | Helper Routes: Health Check & Data Preparation | 2 |
| 2 | Auth: Registration-Verification-Login | 39 |
| 3 | CV | 34 |
| 4 | Post-test Cleanups | 5 |
| 5 | Auth: Logouts and Deletes | 10 |
| 6 | Profile | 3 |
| 7 | Topic | 2 |
| 8 | Projects | 33 |
| 9 | Multimedia | 4 |

One thing about the ordering is worth knowing before you trust a red run.

Cleanup sits at position 4, in the middle, not at the end — folders 5–9 all run
after it. It issues `DELETE /test/cleanup/all/…` for all five test users,
including `vu_user_id`, which folders 5–9 go on to use. Folder 5 then opens with
`Soft Delete User: Login Success (PRE-DELETION)` expecting `200`, and no folder
between them registers a replacement user.

That may be fine — the helper cleanup may not remove what the later folders
need — but it has not been verified here. If folders 5–9 fail on a full run
while passing individually, this is the first place to look.

**Blog endpoints are not covered.** They are the newest module and the only one
with no requests here.

## Running it

Two things are easy to get wrong.

**1. It needs the test-helper routes.** The collection calls `/test/account/random`,
`/test/token/…` and `/test/cleanup/all/…` to mint fixtures and tear them down.
Those routes only exist behind the `test-helpers` feature:

```bash
RUST_ENV=test cargo run --features test-helpers
```

The feature refuses to start under `RUST_ENV=production`.

**2. It hardcodes `http://localhost:8000`.** 131 of the 132 requests carry that
host literally rather than a `{{baseUrl}}` variable, while `PORT` defaults to
`8080` in `.env`, `.env.test` and `.env.example`. Either start the server on
8000:

```bash
PORT=8000 RUST_ENV=test cargo run --features test-helpers
```

…or introduce a `baseUrl` collection variable and rewrite the requests to use
it, which is the more durable fix.

Then run the whole collection, top to bottom:

```bash
newman run postman/CV_API_Test.postman_collection.json
```

## Relationship to the OpenAPI document

They answer different questions and neither replaces the other.

- **What the API accepts and returns** → the OpenAPI document. It is generated
  from the handlers, covers all 53 routes including blog, and a test in
  `src/api/openapi.rs` fails the build if a schema reference goes stale.
  Browsable at `http://localhost:8080/swagger-ui/`, raw at
  `/api-docs/openapi.json`.
- **Whether the API behaves correctly end to end** → this collection.

If you want a Postman collection purely as a request reference, import
`/api-docs/openapi.json` (Postman → Import → Link) instead of copying this one;
it stays current automatically.
