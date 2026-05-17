---
kind: index
status: active
---

# CI And Benchmark Index

This index tracks CI/CD and benchmark job documents. Keep concrete job commands,
trigger rules, artifacts, pass/fail behavior, and upload details in
`doc/development/ci/<job>.md`.

## Strategy

- Use GitHub Actions for formatting, tests, golden checks, and scheduled
  benchmark jobs.
- Store third-party test and benchmark binaries in a separate release
  repository.
- Upload benchmark results to Bencher Cloud after correctness gates pass.
- Never write CI or local benchmark outputs to `target/benchmarks/sox_ng`.

## Jobs

| Job | Status | Trigger | Purpose | Detail |
| --- | --- | --- | --- | --- |
| `test-workspace` | planned | pull request and push | formatting, clippy, Rust tests, doc tests | `ci/test-workspace.md` |
| `benchmark-effects` | planned | scheduled and manual | stable effect benchmark subset | `ci/benchmark-effects.md` |
| `upload-bencher` | planned | scheduled benchmark completion | upload benchmark JSON to Bencher Cloud | `ci/upload-bencher.md` |
