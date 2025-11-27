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
