# backend_actix — working notes

The API behind `port_blog_cms`. Actix Web 4 on SeaORM/Postgres, Redis for the
refresh-token blacklist and rate-limit counters, GCS for uploads. Deployed to
Cloud Run.

**Read [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) before changing anything.**
It has the layering rule, the module map, a request traced end to end, and a
"where does my code go" table.

## Commands

All work from anywhere in the repo — this crate is a member of the workspace
rooted at the repository root.

```bash
cargo run                                    # serve on :8080
cargo run -p migration -- up                 # apply migrations (never automatic)
cargo run -p migration -- status             # what is pending

RUST_TEST_THREADS=1 SKIP_REDIS_TESTS=1 cargo test --workspace --locked
```

The suite is **not parallel-safe** — tests share a process environment and
several call `dotenvy` mid-run. `RUST_TEST_THREADS=1` is required, and
unsetting `REDIS_URL` is *not* enough to skip the Redis integration tests;
`SKIP_REDIS_TESTS=1` is.

## What CI enforces

All five are blocking. Run them before pushing:

```bash
cargo check --workspace --all-targets --locked
RUST_TEST_THREADS=1 SKIP_REDIS_TESTS=1 cargo test --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
```

## Rules that are easy to break

- **`application` may never import `adapter`.** Adapters depend inward; the
  application layer declares traits and `lib.rs` picks the implementations.
  Nothing in the build enforces this — it is enforced by review.
- **A new route needs three edits**: the handler, a registration in
  `lib.rs::init_routes`, and an entry in `src/api/openapi.rs`. A test fails if
  you miss the third.
- **A new error code is a variant in `src/shared/api/error_code.rs`**, not a
  string. Then regenerate the reference:
  `UPDATE_DOCS=1 cargo test -p backend_actix api_errors_doc`.
- **Migrations must stay additive** — see
  [ADR 0003](docs/adr/0003-migrate-before-deploy.md). Dropping a column the
  running build still selects breaks production between the migration and the
  service update.
- **Public items in `ports/`, `shared/`, `api/` and `auth`/`cv`'s `use_cases/`
  must carry a doc comment.** Those layers are under
  `#![deny(missing_docs)]`, so an undocumented struct field or enum variant
  fails the build, not just the review. `cargo check` names every one it
  wants.
- **New code follows the newer module convention** (`topic`, `project`,
  `blog`, `multimedia`), not the older one in `auth` and `cv`. The difference,
  and why, is in `docs/ARCHITECTURE.md`.

## Where the reasoning lives

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — layering, modules, vocabulary
- [`docs/adr/`](docs/adr/) — decisions that would otherwise be re-litigated
- [`docs/API_ERRORS.md`](docs/API_ERRORS.md) — the error-code contract
  (generated; do not hand-edit)
- [`readme.md`](readme.md) — setup, configuration, tests, coverage
- [`build_steps.md`](build_steps.md) — build and deploy
- [`docs/incidents/`](docs/incidents/) — post-mortems

## graphify (optional)

If `graphify-out/graph.json` exists, `graphify query "<question>"` returns a
scoped subgraph that is usually smaller than grepping. `graphify path "<A>"
"<B>"` for relationships, `graphify explain "<concept>"` for one concept.

`graphify-out/` is **gitignored**, so on a fresh clone none of this is
available and none of it is required. Regenerate with:

```bash
graphify extract . --code-only --no-cluster && graphify cluster-only . --no-label
```

Run `graphify update .` after changing code if you are using it.
