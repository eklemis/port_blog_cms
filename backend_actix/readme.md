# backend_actix

The API behind `port_blog_cms` — authentication, blog posts, projects, topics,
CVs and media uploads. Actix Web 4 on SeaORM/Postgres, with Redis for the
refresh-token blacklist and rate-limit counters, and GCS for uploads. Deployed
to Cloud Run.

It owns the database schema through the `migration` crate that lives beside it,
and it is a member of the Cargo workspace rooted at the repository root — so
every `cargo` command below works from anywhere in the repo.

The code is laid out as ports and adapters: each module under `src/modules/`
splits into `adapter/{incoming,outgoing}` and `application/{ports,use_cases,…}`.
**Read [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) before your first
change** — it covers the layering rule, the module map, a request traced end to
end, and where new code belongs.

---

## Prerequisites

| | |
| --- | --- |
| Rust | 1.88 or newer. The Dockerfiles pin `rust:1.88` and CI runs `stable`; there is no `rust-toolchain.toml`, so keep local `stable` at or above the pin. |
| Docker | For the Postgres container. Optional if you already have Postgres 16 on `:5432`. |
| Redis | Required — `REDIS_URL` is mandatory. The compose file does not provide one; see step 2. |
| A GCS service-account key | Only for the media endpoints. Everything else runs without it. |

## Quick start

Five steps from a clean clone to a browsable API.

### 1. Configure

```bash
cp .env.example .env
cp docker/.env.example docker/.env
```

Then edit `.env`: at minimum set `JWT_SECRET` to something at least 32
characters (`openssl rand -base64 48`). The defaults for `DATABASE_URL` and
`REDIS_URL` already match steps 2 and 3.

`.env.example` documents every variable the process reads, which ones are
mandatory, and what the code defaults to when they are unset.

### 2. Start Postgres and Redis

Postgres comes from the compose file in `docker/`:

```bash
docker compose -f docker/postgres.compose.yml up -d
```

Redis is not in that file — start one however you like:

```bash
docker run -d --name cms-redis -p 6379:6379 redis:7
```

Check both are reachable before going further:

```bash
psql "$DATABASE_URL" -c "select 1;"
redis-cli -u "$REDIS_URL" ping
```

### 3. Apply migrations

The server does **not** migrate on startup, so this is a required step, not an
optimisation:

```bash
cargo run -p migration -- up
```

`cargo run -p migration -- status` lists what is applied and what is pending.
Both read `DATABASE_URL` from the environment.

### 4. Run the server

```bash
cargo run
```

It prints `Server run on: 0.0.0.0:8080`.

### 5. Open the API documentation

```
http://localhost:8080/swagger-ui/
```

Every one of the 53 routes is documented there, with request and response
schemas and example values, and you can call them from the page. The raw
document is served at `/api-docs/openapi.json`.

Error responses all share one shape, and the `code` field is a stable
contract — see [`docs/API_ERRORS.md`](docs/API_ERRORS.md) for the full
vocabulary of 49 codes and what each means.

Two probes are also live immediately:

| Endpoint | What it tells you |
| --- | --- |
| `GET /health` | The process is up. No dependency checks. |
| `GET /ready` | Postgres and Redis are both reachable. Use this to confirm step 2 worked. |

---

## Configuration

**`.env.example` is the reference** — it lists all 21 variables the process
reads, grouped by whether the process panics without them, with the defaults
the code actually applies.

Loading works like this: the process reads `RUST_ENV` (default
`development`), tries `.env.<RUST_ENV>`, and falls back to `.env` if that file
does not exist. So `RUST_ENV=test` loads `.env.test`. Real environment
variables always win, which is how Cloud Run injects secrets with no `.env`
file present.

The variables worth calling out here:

| Variable | Required | Default | Purpose |
| --- | --- | --- | --- |
| `DATABASE_URL` | **yes** | — | Postgres connection string. Startup panics without it. |
| `REDIS_URL` | **yes** | — | Token blacklist and rate-limit counters. Startup panics without it. |
| `JWT_SECRET` | **yes** | — | HS256 signing key. Startup panics if shorter than 32 characters. |
| `EMAIL_FROM` | **yes** | — | `From:` address on verification and reset mail. |
| `SMTP_SERVER`, `SMTP_USERNAME`, `SMTP_PASSWORD` | unless `RUST_ENV=test` | — | Outbound mail. Under `RUST_ENV=test` these are ignored in favour of local Mailpit on `SMTP_HOST`/`SMTP_PORT` (`localhost:1025`). |
| `CORS_ALLOWED_ORIGINS` | no | `http://localhost:5173`, `http://127.0.0.1:5173` | Comma-separated browser origins allowed to call the API. **Set this in production** — the fallback is development-only and the server logs a warning when it is used. |
| `PASSWORD_RESET_HANDLER_URL` | no | `0.0.0.0:5173/password-reset` | Frontend route the emailed reset link points at. The token is appended as a path segment. |
| `VERIFICATION_HANDLER_URL` | no | `0.0.0.0:5173/email/verification` | Frontend route the emailed verification link points at. |
| `JWT_PASSWORD_RESET_EXPIRY` | no | `3600` | Reset-token lifetime in seconds. Shorter than verification on purpose — the link is a live credential for the account. |
| `MULTIMEDIA_UPLOAD_BUCKET` | no | `blogport-cms-upload` | GCS bucket that receives uploads. |
| `SKIP_REDIS_TESTS` | no | unset | Set to `1` to skip the Redis integration tests. |

`GOOGLE_APPLICATION_CREDENTIALS` is not an application setting but the media
endpoints need it: signed URLs require a service-account key, so
`gcloud auth application-default login` is not sufficient. Export the path to a
service-account JSON in your shell.

The four `ARGON2_*` / `USE_BLOCKING_HASH` variables appear in `.env.test` but
are currently inert — `main.rs` picks a hardcoded Argon2 profile from
`RUST_ENV` rather than calling `Argon2Hasher::from_env()`.

## Rate limiting

The unauthenticated auth endpoints are limited per caller, backed by Redis:

| Endpoint | Limit |
| --- | --- |
| `POST /api/auth/login` | 10 / 5 min |
| `POST /api/auth/register` | 5 / hour |
| `POST /api/auth/password-reset` | 5 / hour |
| `POST /api/auth/password-reset/{token}` | 10 / hour |
| `POST /api/auth/refresh` | 30 / 5 min |

Authenticated routes are not limited: reaching them already requires a valid
token. The limits are low because each of these costs an Argon2 hash, and
registration also sends mail.

Two things about it are load-bearing and written up as decision records:

- **The limiter fails open when Redis is unreachable** —
  [ADR 0001](docs/adr/0001-rate-limiter-fails-open.md).
- **Callers are keyed on the left-most `X-Forwarded-For` entry**, which is only
  sound behind a proxy that overwrites the header —
  [ADR 0002](docs/adr/0002-rate-limit-keying-on-forwarded-for.md). Read that one
  before putting this service behind anything other than Cloud Run.

---

## Running the tests

```bash
export RUST_TEST_THREADS=1
cargo test -- --nocapture
```

There is a second suite that `cargo test` does not run: an end-to-end Postman
collection with ~889 assertions across auth, CVs, projects, topics, profile and
media. It needs a running server built with the `test-helpers` feature. See
[`postman/README.md`](postman/README.md).

### Coverage

Use `cargo llvm-cov`. Tarpaulin was removed; see below for why.

```bash
export RUST_TEST_THREADS=1
cargo llvm-cov --summary-only --ignore-filename-regex 'src/main\.rs'

# browsable report
cargo llvm-cov --html --ignore-filename-regex 'src/main\.rs'
```

`main.rs` is excluded: it is composition only — `start()`, `init_routes()` and
`main()` wire concrete adapters into `AppState`. Covering it means booting the
process against a real database and Redis, which the suite deliberately does
not do.

Run without `SKIP_REDIS_TESTS=1` for a true figure. Skipping the Redis
integration tests leaves `token_repository_redis.rs` reading as ~0% when it is
in fact covered.

#### Current coverage and what is excluded

As of the last measurement: **94.68% line coverage** overall, **92.26% counting
production code only** (3324/3603 lines).

llvm-cov measures the test binary, so `#[cfg(test)]` modules are included in the
overall figure. Those are largely self-covering, which flatters it — the
production-only number is the one worth tracking.

Of the 279 uncovered production lines, 101 cannot be reached by a unit test:

| Lines | Area | Why |
| --- | --- | --- |
| 38 | `token_repository_redis.rs`, `rate_limit/redis_store.rs` | Need a reachable Redis |
| 55 | `*/sea_orm_entity/*` | `DeriveEntityModel` output — `Relation`, `ActiveModel` |
| 8 | `smtp_sender::new_local` | Constructs a live SMTP transport |

Excluding those, production coverage is **94.92%**. The Redis lines are covered
by the integration tests below, which need a real instance; they read as
uncovered whenever `SKIP_REDIS_TESTS=1` is set.

The remaining 178 are spread across roughly 60 files at a median of about 3
lines each — mostly error-mapping arms. Closing them adds tests that move a
number without catching regressions, so they are left deliberately.

Two things excluded on purpose that should stay that way:

- `main.rs` — composition only; covering it means booting the process.
- `LogoutRequestError` — an enum with no variants, so its `Display` impl cannot
  execute. Unreachable by construction, not an oversight.

#### Why not tarpaulin

It could not attribute `async fn` bodies inside `#[async_trait]`, which 167
files here use, and it clobbered the proc-macro dylibs rust-analyzer caches.
Both are reproducible; see
[ADR 0004](docs/adr/0004-llvm-cov-over-tarpaulin.md). Treat any tarpaulin
percentage in an old commit or issue as unreliable.

### Integration tests (need real services)

Some paths cannot be exercised with a mock. They are gated so the suite stays
green without them, and they read as uncovered when skipped.

```bash
# Redis: token blacklist + rate-limit counters
export RUST_TEST_THREADS=1
REDIS_URL='rediss://...' cargo test
```

`token_repository_redis` covers the blacklist; `rate_limit::redis_store` covers
the INCR / EXPIRE / TTL sequence. The middleware tests use an in-memory store
that reimplements the counting, so they cannot catch a mistake in the Redis
commands themselves — only these can. They also assert the window does not
slide when a caller keeps exceeding the limit, which would otherwise let someone
hold their own window open indefinitely.

Set `SKIP_REDIS_TESTS=1` to skip them. Note that unsetting `REDIS_URL` alone is
not enough, because other tests call `dotenvy` mid-run and repopulate it.

### Run the test without a reachable Redis
The `token_repository_redis` tests are integration tests that need a live Redis.
Skip them explicitly when you have none:
```bash
export RUST_TEST_THREADS=1
SKIP_REDIS_TESTS=1 cargo test
```
Note: unsetting `REDIS_URL` is not enough on its own — other tests in the binary
call `dotenvy`, which loads `.env` into the shared process environment partway
through a run.


---

## Handy commands

Open a psql shell against the compose container:

```bash
docker exec -it cms-postgres psql -d cms -U developer
```

Run the server with the test-only helper routes, in release mode:

```bash
RUST_ENV=test cargo run --release --features test-helpers
```

The `test-helpers` feature refuses to start under `RUST_ENV=production`.

Save the OpenAPI document from a running server, e.g. to diff it against a
previous release:

```bash
curl -s http://localhost:8080/api-docs/openapi.json | jq . > openapi.json
```

## Deploying

See [`build_steps.md`](build_steps.md). Short version, from this directory:

```bash
./build.sh && ./deploy.sh
```

`build.sh` builds and pushes the image (~15–20 min); `deploy.sh` applies
pending migrations, then updates the Cloud Run service (~1–2 min). Migrations
run *before* the service update and a failure aborts the deploy — see
`build_steps.md` for why that ordering matters.
