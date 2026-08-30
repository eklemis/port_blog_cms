### Run the test
```bash
export RUST_TEST_THREADS=1
cargo test -- --nocapture
```

### Run the test and see coverage with tarpauline (Preferable)
```bash
cargo tarpaulin --ignore-tests --out Html --line
```
or using llvm-cov 
```bash
cargo llvm-cov --html
```

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
