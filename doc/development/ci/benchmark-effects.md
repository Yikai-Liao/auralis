---
kind: ci-benchmark-job
job: "benchmark-effects"
status: planned
trigger: "scheduled,manual"
runner: "GitHub Actions Ubuntu"
owner: "performance"
upload_target: "GitHub Actions artifacts and Bencher Cloud"
---

# Effect Benchmark CI Job

## One-Line Goal

Run a stable subset of whole-buffer effect benchmarks without overwriting the
preserved local SoX-ng baseline.

## Inputs

- case set: stable implemented effects and selected SIMD/DSP kernels;
- audio shape: fixed deterministic mono/stereo fixtures;
- backend: scalar and SIMD variants where supported;
- third-party binaries: `sox_ng` only where the effect still uses it as a
  valid baseline;
- feature flags: release build, all features.

## Command

```bash
cd tools/pytest
rtk uv run python ../benchmark_sox_ng_effects.py \
  --repo-root ../.. \
  --output-dir ../../target/benchmarks/ci/${GITHUB_RUN_ID}/effects
```

## Output Policy

- output directory: `target/benchmarks/ci/${GITHUB_RUN_ID}/effects`;
- preserved baseline: never write to `target/benchmarks/sox_ng`;
- artifacts: `report.json`, `report.md`, raw command log;
- retention: workflow artifact plus Bencher upload input.

## Pass/Fail Rule

Correctness mismatches fail. Performance regressions are report-only until a
stable threshold and confirmation policy are defined.

## Rerun Rule

Any claimed speedup must hold on a second `--skip-build` confirmation run
before being treated as real performance evidence.

## Failure Triage

- missing binary: fail if the selected case declares it required;
- unsupported case: explicit skip with `skip_reason`;
- noisy timing: rerun before changing thresholds;
- benchmark script failure: preserve logs and `report.json` if present.

## Done When

- Output path is unique per run.
- Preserved benchmark baseline is untouched.
- Reports are uploaded.
- Bencher upload has a stable input artifact.
