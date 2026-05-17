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
| `test-workspace` | planned | pull request and push | formatting, clippy, Rust tests, doc tests | migration target: `ci/test-workspace.md` |
| `golden-sox-ng` | planned | pull request, push, and release/gnhf gate | required SoX-ng golden checks with missing-oracle failure in release contexts | migration target: `ci/golden-sox-ng.md` |
| `external-binary-release` | planned | manual release in dependency repository | publish third-party benchmark/test binaries such as `sox_ng` | migration target: `ci/external-binary-release.md` |
| `benchmark-effects` | planned | scheduled and manual | stable effect benchmark subset using fresh output directories | migration target: `ci/benchmark-effects.md` |
| `upload-bencher` | planned | scheduled benchmark completion | upload benchmark JSON to Bencher Cloud | migration target: `ci/upload-bencher.md` |

Rows marked `migration target` preserve CI scope before flat job documents are
written. Create each job document from
`doc/development/templates/ci-benchmark-job.md` when migrating that job.
