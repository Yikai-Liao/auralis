---
kind: ci-benchmark-job
job: "<job-name>"
status: planned
trigger: "<nightly|manual|pull-request>"
runner: "GitHub Actions Ubuntu"
owner: "<area>"
upload_target: "Bencher Cloud"
---

# CI Benchmark Job Template

Use this for one CI or scheduled benchmark job. Put document metadata in the
YAML front matter above. Keep the filled file under 250 lines.

## One-Line Goal

Example:

Run a stable subset of whole-buffer effect benchmarks and upload JSON results
without overwriting local baseline directories.

## Inputs

- case set:
- audio shape:
- backend:
- third-party binaries:
- feature flags:

Example:

- case set: `gain,by=-3dB`, `mix.sum`, `finite_peak_rms`
- audio shape: 48 kHz stereo, fixed duration per case
- backend: scalar and SIMD where supported
- third-party binaries: `sox_ng` only for cases where it is a valid oracle
- feature flags: default plus SIMD job variant

## Command

Use a real repo command.

```bash
rtk uv run --directory tools/pytest python ../benchmark_sox_ng_effects.py \
  --repo-root ../.. \
  --effect gain \
  --output-dir ../../target/benchmarks/ci/${GITHUB_RUN_ID}/gain
```

## Output Policy

- output directory:
- preserved baseline:
- artifacts:
- retention:

Example:

- output directory: `target/benchmarks/ci/${GITHUB_RUN_ID}/...`
- preserved baseline: never write to `target/benchmarks/sox_ng`
- artifacts: `report.json`, `report.md`, raw tool log
- retention: workflow artifact and Bencher upload

## Pass/Fail Rule

Example:

Correctness mismatches fail. Performance regressions are report-only until a
stable threshold and confirmation rerun policy are defined.

## Rerun Rule

Example:

Any claimed speedup must hold on a second `--skip-build` confirmation run
before it is treated as real performance evidence.

## Failure Triage

- missing binary:
- unsupported case:
- noisy timing:
- benchmark script failure:

Example:

Missing required third-party binary fails jobs that declare it required.
Unsupported cases must be explicit skips with a reason in `report.json`.

## Done When

- Command runs from repo root with `rtk`.
- Output directory never overwrites preserved baselines.
- Artifacts are named and retained.
- Upload target is documented.
- Pass/fail and rerun rules are unambiguous.
