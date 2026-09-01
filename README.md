# port_blog_cms

A portfolio/blog CMS split across four deployed services and one frontend. Everything
lives in this one repository; there are no sibling repos to clone.

## Layout

| Path | Stack | What it is | Deploys to |
| --- | --- | --- | --- |
| `backend_actix/` | Rust · Actix Web · SeaORM | The API. Auth, blogs, projects, CVs, uploads. Owns the Postgres schema via its `migration` crate. | Cloud Run |
| `image-processor-function/` | Rust · Actix Web | Triggered by Eventarc on GCS `object.finalized`. Resizes uploads and writes variants to the ready bucket. | Cloud Run |
| `media-status-updater/` | Node 22 | GCS-triggered function that marks the media row in Postgres once variants land. | Cloud Functions |
| `blogport_frontend/` | SvelteKit 2 · Svelte 5 · Tailwind 4 | The web client. Talks to `backend_actix` through its own `/api` server routes. | — |
| `backend_node/` | — | Empty stub: a lone `package.json`, no source. Kept only so the name stays reserved. | — |

`backend_actix/` also contains two library crates it owns: `entity/` (SeaORM models)
and `migration/` (schema migrations).

## The Rust workspace

The four Rust crates form **one Cargo workspace rooted at the repository root**
(`./Cargo.toml`). That means:

- **One `Cargo.lock`, committed.** A dependency resolves to the same version in every
  crate, and a Cloud Build produces the same graph as a local build. Use `--locked` in
  CI and in Docker so a stale lockfile fails the build instead of being silently
  re-resolved.
- **One `target/` at the repository root.** Building one service warms the cache for
  the other. Before the workspace existed these were separate and held ~17.5 GB
  between them.
- **Release profile settings live only in `./Cargo.toml`.** Cargo ignores
  `[profile.*]` in member crates. `image-processor-function` keeps its smaller
  binary through a `[profile.release.package.…]` override.

Common commands, all from the repository root:

```bash
cargo check --workspace --all-targets --locked
cargo build --release --locked -p backend_actix
cargo build --release --locked -p image-processor-function
```

### Two GCS client versions on purpose

`backend_actix` uses `google-cloud-storage` 1.x and `image-processor-function` uses
0.22. They are semver-incompatible, so the lockfile carries both. That is expected —
not a conflict to resolve.

## Running the tests

```bash
export RUST_TEST_THREADS=1
SKIP_REDIS_TESTS=1 cargo test --workspace --locked
```

Drop `SKIP_REDIS_TESTS=1` and set `REDIS_URL` to also run the Redis integration tests.
Unsetting `REDIS_URL` alone does **not** skip them — other tests call `dotenvy`
mid-run and repopulate the process environment. See `backend_actix/readme.md` for
coverage details and why tarpaulin was dropped in favour of `cargo llvm-cov`.

Frontend:

```bash
cd blogport_frontend && bun install && bun run test
```

## Frontend development

```bash
cd blogport_frontend
bun install
echo 'VITE_BACKEND_BASE_URL=http://localhost:8080' > .env
bun run dev
```

`VITE_BACKEND_BASE_URL` is the only variable it needs; it defaults to
`http://localhost:8080` when unset.

## Deploying

Each service deploys independently — there is no orchestrated all-at-once deploy.

```bash
cd backend_actix           && ./build.sh && ./deploy.sh
cd image-processor-function && ./deploy.sh
cd media-status-updater     && ./deploy.sh
```

**The two Rust services build with the repository root as the Docker build context**,
because their crates need the root `Cargo.toml` and `Cargo.lock` to resolve. Their
Dockerfiles are therefore selected by a `cloudbuild.yaml` rather than by
`gcloud builds submit --tag`, and the scripts resolve the repo root themselves — you
can still run them from inside the service directory. The root `.gcloudignore` and
`.dockerignore` keep the frontend and the Node service out of the upload.

To build one locally:

```bash
docker build -f backend_actix/Dockerfile -t backend-actix .
```

Note the trailing `.` — the context is the root, not the service directory.
