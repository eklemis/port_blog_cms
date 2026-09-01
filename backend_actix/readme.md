### Run the test
```bash
export RUST_TEST_THREADS=1
cargo test -- --nocapture
```

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

Tarpaulin cannot attribute the body of an `async fn` inside `#[async_trait]`,
and 167 files here use that macro. It reported `reset_password.rs` at 5/32
lines with lines 105-106 uncovered — while a passing test asserts the value
those lines write. Its headline of 69.58% against llvm-cov's 91.57% is mostly
that artifact.

It also builds into `target/debug` with different flags, replacing the
proc-macro dylibs that rust-analyzer caches paths to. That produces

    proc-macro panicked: failed to load macro: Cannot create expander for
    .../libasync_trait-<hash>.dylib: No such file or directory

in the editor after every run. `cargo llvm-cov` builds into
`target/llvm-cov-target` instead and leaves the normal build alone.

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

## Environment variables

| Variable | Required | Default | Purpose |
| --- | --- | --- | --- |
| `CORS_ALLOWED_ORIGINS` | no | `http://localhost:5173`, `http://127.0.0.1:5173` | Comma-separated browser origins allowed to call the API. **Set this in production** — the fallback is development-only. |
| `SKIP_REDIS_TESTS` | no | unset | Set to `1` to skip the Redis integration tests. |

### Rate limiting

The unauthenticated auth endpoints are limited per caller, backed by Redis:

| Endpoint | Limit |
| --- | --- |
| `POST /api/auth/login` | 10 / 5 min |
| `POST /api/auth/register` | 5 / hour |
| `POST /api/auth/password-reset` | 5 / hour |
| `POST /api/auth/password-reset/{token}` | 10 / hour |
| `POST /api/auth/refresh` | 30 / 5 min |

Authenticated routes are not limited: reaching them already requires a
valid token. The limits are low because each of these costs us an Argon2
hash, and registration also sends mail — they are a denial-of-service
lever as much as a credential-guessing one.

Callers are keyed on the left-most `X-Forwarded-For` entry, falling back to
the peer address. Behind Cloud Run the peer address is the load balancer for
every request, so keying on it would collapse all clients onto one counter
and let one busy caller lock out everybody. **This is only sound behind a
proxy that overwrites the header.** Cloud Run does; a proxy that merely
appends would let a caller rotate the value for a fresh bucket per request.

If Redis is unreachable the limiter fails open and logs. Refusing every
login during a cache outage would turn it into a total authentication
outage; the limiter is a mitigation, not the security boundary.

| `PASSWORD_RESET_HANDLER_URL` | no | `0.0.0.0:5173/password-reset` | Frontend route the emailed reset link points at. The token is appended as a path segment. |
| `JWT_PASSWORD_RESET_EXPIRY` | no | `3600` | Reset-token lifetime in seconds. Shorter than verification on purpose — the link is a live credential for the account. |

## Run server with `test-helpers` flag and release version
```bash
RUST_ENV=test cargo run --release --features test-helpers
```

## Open postgres database cms from terminal
```bash
docker exec -it postgres-db psql -d cms -U developer
```

## Create postgres docker container
```bash

```
