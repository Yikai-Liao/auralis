---
kind: ci-benchmark-job
job: "test-workspace"
status: planned
trigger: "pull-request,push"
runner: "GitHub Actions Ubuntu"
owner: "workspace"
upload_target: "GitHub Actions artifacts"
---

# Test Workspace CI Job

## One-Line Goal

Run the standard Rust and helper-tool checks that prove a change is ready for
review.

## Inputs

- case set: formatting, clippy, Rust unit/integration tests, doc tests, and
  selected helper tests;
- audio shape: deterministic small fixtures already checked into the repo;
- backend: default backend selection;
- third-party binaries: none required for the base job;
- feature flags: workspace all features.

## Command

```bash
rtk cargo fmt --all --check
rtk cargo clippy --workspace --all-targets --all-features -- -D warnings
rtk cargo test --workspace --all-features
rtk cargo test --doc --workspace
cd tools/pytest
rtk uv sync
rtk uv run pytest
```

## Output Policy

- output directory: normal Cargo and pytest outputs;
- preserved baseline: not applicable;
- artifacts: test logs and failure artifacts only;
- retention: GitHub Actions default retention.

## Pass/Fail Rule

Formatting, clippy, Rust tests, doc tests, and helper pytest failures fail the
job. Golden tests that require external binaries belong to `golden-sox-ng`.

## Rerun Rule

Rerun failed jobs after fixing the code or docs. No performance conclusion can
be drawn from this job.

## Failure Triage

- missing Rust toolchain: fail infrastructure setup;
- clippy warning: fix or justify in code;
- helper pytest dependency issue: run through `uv`, not system Python;
- flaky test: quarantine only with a linked issue and replacement coverage.

## Done When

- Commands run from repo root with `rtk`.
- `tools/pytest` uses `uv`.
- The job excludes required external-oracle checks.
- Logs are retained for failures.
