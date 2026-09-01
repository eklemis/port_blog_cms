# 0004 — Coverage is measured with `cargo llvm-cov`, not tarpaulin

**Status:** Accepted
**Component:** test tooling

## Context

Tarpaulin was the original coverage tool. Its numbers did not survive scrutiny.

## Decision

**Coverage is measured with `cargo llvm-cov`. Tarpaulin was removed.**

```bash
export RUST_TEST_THREADS=1
cargo llvm-cov --summary-only --ignore-filename-regex 'src/main\.rs'
```

## Consequences

Two concrete reasons, both reproducible:

**It cannot attribute the body of an `async fn` inside `#[async_trait]`, and 167
files here use that macro.** It reported `reset_password.rs` at 5/32 lines with
lines 105–106 uncovered — while a passing test asserts the value those lines
write. Its headline of 69.58% against llvm-cov's 91.57% was mostly that
artifact. A coverage tool that under-reports by construction is worse than none:
it sends people to write tests for code that is already tested.

**It builds into `target/debug` with different flags**, replacing the proc-macro
dylibs that rust-analyzer caches paths to. That produced

```
proc-macro panicked: failed to load macro: Cannot create expander for
.../libasync_trait-<hash>.dylib: No such file or directory
```

in the editor after every run. `cargo llvm-cov` builds into
`target/llvm-cov-target` and leaves the normal build alone.

llvm-cov measures the test binary, so `#[cfg(test)]` modules count toward the
headline figure and flatter it. The production-only number is the one worth
tracking. `readme.md` carries the current figures, what is excluded and why.

Anyone reading an old commit or issue that cites a tarpaulin percentage should
treat it as unreliable rather than as evidence of a regression.

## Alternatives considered

**Keep both.** Rejected: two numbers that disagree by twenty points invite
arguing about the tool instead of the tests.
