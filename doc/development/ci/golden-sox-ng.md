---
kind: ci-benchmark-job
job: "golden-sox-ng"
status: planned
trigger: "pull-request,push,release"
runner: "GitHub Actions Ubuntu"
owner: "testing"
upload_target: "GitHub Actions artifacts"
---

# SoX-ng Golden CI Job

## One-Line Goal

Run required SoX-ng golden checks with a real `sox_ng` binary and fail release
contexts when the oracle is missing.

## Inputs

- case set: Rust golden tests plus Python golden helper tests while they remain;
- audio shape: deterministic L0 golden fixtures;
- backend: default backend selection;
- third-party binaries: `sox_ng` from the external binary release repository;
- feature flags: workspace all features.

## Command

```bash
export AURALIS_SOX_NG_BIN=${AURALIS_SOX_NG_BIN:-/opt/auralis-tools/sox_ng}
rtk cargo test --workspace --all-features golden
cd tools/pytest
rtk uv run pytest -m golden
```

## Output Policy

- output directory: test temp directories only;
- preserved baseline: never write to `target/benchmarks/sox_ng`;
- artifacts: golden diff summaries, decoded metric reports, failing fixtures;
- retention: GitHub Actions artifacts for failed runs.

## Pass/Fail Rule

Correctness mismatches fail. Missing `sox_ng` fails release and gnhf gates. Pull
request behavior may use a clear infrastructure failure rather than a silent
skip.

## Rerun Rule

Rerun after changing fixtures, oracle binary version, or tolerance. Record the
oracle binary version in artifacts.

## Failure Triage

- missing binary: fail setup and point to `external-binary-release`;
- unsupported case: skip only with explicit `skip_reason`;
- tolerance mismatch: inspect decoded sample metrics before changing tolerance;
- Python helper failure: keep Rust/testkit as the authoritative target where
  migration exists.

## Done When

- CI installs a real `sox_ng` binary.
- Missing oracle cannot look like green release coverage.
- Golden diff artifacts are retained.
- Unsupported cases are explicit skips with reasons.
