# port_blog_cms — working notes

A portfolio/blog CMS: four deployed services and one frontend, in one
repository. See [`README.md`](README.md) for the layout table and the workspace
rationale.

| Path | What it is |
| --- | --- |
| `backend_actix/` | The API. Rust · Actix Web · SeaORM. Owns the Postgres schema. Has its own [`CLAUDE.md`](backend_actix/CLAUDE.md) — **read it before working there.** |
| `image-processor-function/` | Rust. Eventarc-triggered; resizes uploads. |
| `media-status-updater/` | Node 22. Marks media ready once variants land. |
| `blogport_frontend/` | The client. SvelteKit 2 · Svelte 5 (runes) · Tailwind 4 · Feature-Sliced · TDD. Has its own [`CLAUDE.md`](blogport_frontend/CLAUDE.md) — **read it before working there.** |
| `backend_node/` | Empty stub. Name reserved, no source. |
| `design/` | Figma tooling. `figma-design-lint/` checks the design file for the silent layout failures that survive review — import it from its manifest. |

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

## Working copies — who edits where

Several checkouts of this repository share one `.git`. Two sessions working in
the same one has already cost real work twice: uncommitted changes swept into
somebody else's commit, and a branch switching under an editor mid-task.

| Directory | Whose | Notes |
| --- | --- | --- |
| `port_blog_cms` | the human | The original checkout. Agents: read it, do not switch its branch or stage in it. |
| `port_blog_cms-frontend-wt` | the frontend session | Holds `main`. |
| `port_blog_cms-backend-wt` | the backend session | Kept on a **detached** checkout, so it never contends for a branch name. |
| `blogport-server-run` | nobody | Not a checkout. The binary serving `:8080` lives here so no branch operation can change what is running. |

Rules that follow from it:

- **Only one worktree can hold a given branch.** `git checkout main` fails if
  another worktree has it. Work on your own branch, or detached.
- **Never `git add -A` / `git commit -a` in a shared worktree.** Stage the paths
  you touched by name: `git commit -- path/one path/two`. An index you did not
  fill is somebody else's work in progress.
- **Uncommitted changes you did not write are not yours to discard.** Preserve
  them first — `git stash create` gives a commit you can point a branch at —
  then say where you put it.
- **Do not serve from a worktree.** Copy the binary to `blogport-server-run`;
  see its README.

## Things that will bite

- **The lockfile must stay buildable on `rust:1.88`**, which both Dockerfiles
  pin and which is older than a typical local `stable`. A careless
  `cargo update` can pull a crate needing a newer rustc and break the container
  build while everything passes locally.
- **One lockfile per JS service, and it decides the package manager.** Vercel
  and Cloud Build pick their package manager from whichever lockfile they find,
  so a second one silently changes how a service builds. `blogport_frontend`
  uses **Bun** (`bun.lockb`) — run `bun install` there, never `npm install`.
- **Both Rust services build with the repository root as the Docker context**,
  because their crates need the root manifest and lockfile. Their Dockerfiles
  are selected by `cloudbuild.yaml` rather than `gcloud builds submit --tag`.
- **Two `google-cloud-storage` majors coexist on purpose** (1.x and 0.22).
  Expected, not a conflict to resolve.
- **The test suite is not parallel-safe.** `RUST_TEST_THREADS=1` always.

## Licence

None, deliberately — all rights reserved. The repository is public so the code
can be read, not reused. Do not add a LICENSE file or licence headers; see the
Licence section of `README.md`.

## Deploying

Each service deploys independently; there is no orchestrated all-at-once deploy.

```bash
cd backend_actix            && ./build.sh && ./deploy.sh
cd image-processor-function && ./deploy.sh
cd media-status-updater     && ./deploy.sh
```
