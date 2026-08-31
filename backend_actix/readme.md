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
| `CORS_ALLOWED_ORIGINS` | no | `http://localhost:5177`, `http://127.0.0.1:5177` | Comma-separated browser origins allowed to call the API. **Set this in production** — the fallback is development-only. |
| `SKIP_REDIS_TESTS` | no | unset | Set to `1` to skip the Redis integration tests. |
| `PASSWORD_RESET_HANDLER_URL` | no | `0.0.0.0:5177/password-reset` | Frontend route the emailed reset link points at. The token is appended as a path segment. |
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
