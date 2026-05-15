# Auralis Development Commands

Commands below are intended to be run from the repository root unless a block
explicitly changes directory. Local shell commands should use `rtk`.

## Rust checks

```bash
rtk cargo fmt --all --check
rtk cargo clippy --workspace --all-targets --all-features -- -D warnings
rtk cargo test --workspace --all-features
rtk cargo test --doc --workspace
rtk cargo bench
```

## Python tests

Python test tooling must run through the project `uv` environment. Do not use
ad-hoc system `pip` installs for project tests.

```bash
cd tools/pytest
rtk uv sync
rtk uv run pytest
```

## SoX-ng golden validation

Release and golden validation must run with a real SoX-ng oracle.

```bash
export AURALIS_SOX_NG_BIN=${AURALIS_SOX_NG_BIN:-/usr/local/bin/sox_ng}
rtk cargo test --workspace --all-features golden
cd tools/pytest
rtk uv run pytest -m golden
```

Set `AURALIS_REQUIRE_SOX_NG=1` when a broader Rust test invocation must fail
instead of skipping SoX-ng oracle checks.

## SoX-ng effect benchmarks

The preserved baseline is `target/benchmarks/sox_ng`. Do not use that path as a
new benchmark `--output-dir`; write every new run to a fresh directory under
`target/benchmarks/`.

List benchmarkable effect cases:

```bash
cd tools/pytest
rtk uv run python ../benchmark_sox_ng_effects.py --repo-root ../.. --list-cases
```

Run one effect benchmark:

```bash
cd tools/pytest
rtk uv run python ../benchmark_sox_ng_effects.py \
  --repo-root ../.. \
  --effect stats \
  --output-dir ../../target/benchmarks/sox_ng_stats_current \
  --iterations 5 \
  --warmups 1 \
  --duration-seconds 90
```

Run the full implemented-effects benchmark suite:

```bash
cd tools/pytest
rtk uv run python ../benchmark_sox_ng_effects.py \
  --repo-root ../.. \
  --output-dir ../../target/benchmarks/sox_ng_full_current \
  --iterations 5 \
  --warmups 1 \
  --duration-seconds 90
```

Confirm a promising result with a second run. Use `--skip-build` only after the
first run has built `target/release/auralis`.

```bash
cd tools/pytest
rtk uv run python ../benchmark_sox_ng_effects.py \
  --repo-root ../.. \
  --effect stats \
  --output-dir ../../target/benchmarks/sox_ng_stats_confirm \
  --iterations 5 \
  --warmups 1 \
  --duration-seconds 90 \
  --skip-build
```

Each benchmark output directory contains `report.json` and `report.md`.

## Coverage report

```bash
rtk python3 tools/check_layered_coverage.py --report target/layered-coverage/report.json
```
