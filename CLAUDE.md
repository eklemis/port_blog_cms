# port_blog_cms — working notes

A portfolio/blog CMS: four deployed services and one frontend, in one
repository. See [`README.md`](README.md) for the layout table and the workspace
rationale.

| Path | What it is |
| --- | --- |
| `backend_actix/` | The API. Rust · Actix Web · SeaORM. Owns the Postgres schema. Has its own [`CLAUDE.md`](backend_actix/CLAUDE.md) — **read it before working there.** |
| `image-processor-function/` | Rust. Eventarc-triggered; resizes uploads. |
| `media-status-updater/` | Node 22. Marks media ready once variants land. |
| `blogport_frontend/` | SvelteKit 2 · Svelte 5 · Tailwind 4. |
| `backend_node/` | Empty stub. Name reserved, no source. |

## The Rust workspace

The four Rust crates are **one Cargo workspace rooted here**. One `Cargo.lock`,
one `target/`. Use `--locked` so a stale lockfile fails the build instead of
being silently re-resolved.

```bash
cargo check --workspace --all-targets --locked
RUST_TEST_THREADS=1 SKIP_REDIS_TESTS=1 cargo test --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Those four, plus `cargo doc` under `RUSTDOCFLAGS="-D warnings"`, are what CI
blocks on.

## Things that will bite

- **The lockfile must stay buildable on `rust:1.88`**, which both Dockerfiles
  pin and which is older than a typical local `stable`. A careless
  `cargo update` can pull a crate needing a newer rustc and break the container
  build while everything passes locally.
- **`package-lock.json` is the only lockfile that belongs in the repo.** Vercel
  and Cloud Build pick their package manager from whichever lockfile they find.
- **Both Rust services build with the repository root as the Docker context**,
  because their crates need the root manifest and lockfile. Their Dockerfiles
  are selected by `cloudbuild.yaml` rather than `gcloud builds submit --tag`.
- **Two `google-cloud-storage` majors coexist on purpose** (1.x and 0.22).
  Expected, not a conflict to resolve.
- **The test suite is not parallel-safe.** `RUST_TEST_THREADS=1` always.

## Deploying

Each service deploys independently; there is no orchestrated all-at-once deploy.

```bash
cd backend_actix            && ./build.sh && ./deploy.sh
cd image-processor-function && ./deploy.sh
cd media-status-updater     && ./deploy.sh
```
