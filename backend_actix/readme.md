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
