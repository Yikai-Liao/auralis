# Auralis Development Guide

This file is the stable development index and current priority guide for
Auralis. Detailed plans live under [`doc/development/`](doc/development/), and
the target document layout is defined in
[`doc/development/README.md`](doc/development/README.md).

The core rule:

> Implement one feature at a time. Finish its tests, documentation, examples,
> and comparison policy before starting the next feature.

---

## Current Priority

The active priority is keeping development planning in the short target
documents with clear ownership. The old numbered roadmap Markdown files have
been retired; do not recreate them for new work.

Work in this order:

1. Keep the new path map in
   [`doc/development/README.md`](doc/development/README.md) authoritative.
2. Use [`graph-engine.md`](doc/development/graph-engine.md),
   [`cli.md`](doc/development/cli.md), and
   [`op-registration.md`](doc/development/op-registration.md) for the shared
   Graph Execution Engine, lightweight CLI, and operator registry plans.
3. Use [`testing.md`](doc/development/testing.md),
   [`ci.md`](doc/development/ci.md), and
   [`oracles.md`](doc/development/oracles.md) for testing gates, CI/CD,
   benchmark dependencies, and oracle replacement work.
4. Use [`effects.md`](doc/development/effects.md),
   [`dsp.md`](doc/development/dsp.md), and
   [`simd.md`](doc/development/simd.md) as indexes, then create flat detail
   files from the templates as each effect, primitive, or kernel is planned.
5. Keep [`python.md`](doc/development/python.md) at `not-planned` until the
   Rust graph, effect, parameter, error, and buffer contracts stabilize.

---

## Active Development Tracks

| Track | Authoritative file | Current status |
| --- | --- | --- |
| Path map and doc rules | [`doc/development/README.md`](doc/development/README.md) | active |
| Graph Execution Engine | [`doc/development/graph-engine.md`](doc/development/graph-engine.md) | active |
| CLI adapter | [`doc/development/cli.md`](doc/development/cli.md) | active |
| Operator registration | [`doc/development/op-registration.md`](doc/development/op-registration.md) | active |
| Testing development | [`doc/development/testing.md`](doc/development/testing.md) | active |
| CI and benchmarks | [`doc/development/ci.md`](doc/development/ci.md) | active |
| Oracle replacement | [`doc/development/oracles.md`](doc/development/oracles.md) | active |
| Effects | [`doc/development/effects.md`](doc/development/effects.md) | active |
| DSP primitives | [`doc/development/dsp.md`](doc/development/dsp.md) | active |
| SIMD kernels | [`doc/development/simd.md`](doc/development/simd.md) | active |
| Formats and codecs | [`doc/development/formats.md`](doc/development/formats.md) | active |
| Python bindings | [`doc/development/python.md`](doc/development/python.md) | not-planned |

Root and stable reference documents keep their narrower roles:

| File | Role |
| --- | --- |
| [`README.md`](README.md) | user-facing entry point and compact current status |
| [`doc/status.md`](doc/status.md) | current implemented capability snapshot |
| [`doc/architecture.md`](doc/architecture.md) | stable architecture and crate-boundary reference |
| [`doc/testing.md`](doc/testing.md) | stable L0-L7 testing contract |
| [`doc/development-commands.md`](doc/development-commands.md) | runnable command reference only |
| [`AGENTS.md`](AGENTS.md) | repository collaboration rules |

---

## Architecture Direction

The shared execution model is the Graph Execution Engine:

- CLI input lowers into structured `GraphRequest` values.
- `auralis-graph` validates `GraphRequest` into `ValidGraph`.
- Planning produces `ExecutionPlan`.
- Execution is whole-buffer offline rendering only.
- Graph code must not parse CLI strings.
- Parallelism may use deterministic DAG-level, channel-level, or
  buffer-internal data-parallel work.

Do not introduce streaming, chunked, or realtime graph execution goals.

The target operator model is:

- a thin `auralis-op` contract crate;
- distributed compile-time registration with deterministic catalog ordering;
- duplicate op and alias checks in tests;
- named parameters across CLI, graph specs, Rust helpers, and future bindings.

The target CLI model is a lightweight adapter:

- small command surface;
- comma-separated named `--fx` parameters such as `gain,by=-3dB`;
- global execution-mode flags such as `--check` and `--plan`;
- no new SoX-style positional parameter surface;
- no duplicated plan-mode parsing path.

---

## Feature Completion Rules

Each feature should be one focused unit of work. A feature is complete only when
the implementation, tests, documentation, and verification that apply to that
feature are present in the same commit or clearly recorded as a blocker in the
target development document.

Required checks scale with scope:

| Feature type | Required evidence |
| --- | --- |
| Library API | unit tests and doc tests |
| Format I/O | round-trip tests, unsupported-format tests, and format docs |
| Effect | analytical tests, oracle policy, effect docs, and golden tests when applicable |
| DSP primitive | mathematical contract, callers, tests, and benchmark plan if hot |
| SIMD kernel | scalar differential tests, tail tests, fallback behavior, and benchmark plan |
| CLI | command tests, diagnostics, and library-equivalent behavior tests |
| CI job | trigger, commands, artifacts, pass/fail rules, and dependency policy |

Every public API must document purpose, parameter units, error behavior,
determinism, examples, and numerical behavior where relevant.

---

## Testing Contract

[`doc/testing.md`](doc/testing.md) is the stable testing contract. Development
work that changes the test strategy belongs in
[`doc/development/testing.md`](doc/development/testing.md).

Use the L0-L7 vocabulary consistently:

- L0 deterministic corpus
- L1 format I/O correctness
- L2 oracle or golden regression
- L3 analytical DSP tests
- L4 property and metamorphic tests
- L5 chunk-invariance where a chunked internal state is intentionally exposed
- L6 scalar-vs-SIMD differential tests
- L7 fuzzing, sanitizers, and coverage

If a layer does not apply, document the narrow reason. Do not use
`not_applicable` to hide untested behavior.

---

## Command Gates

Use `rtk` for shell commands in this repository.

For ordinary Rust changes, run the smallest relevant subset first, then broaden
when the touched surface requires it:

```bash
rtk cargo fmt --all --check
rtk cargo clippy --workspace --all-targets --all-features -- -D warnings
rtk cargo test --workspace --all-features
rtk cargo test --doc --workspace
```

For CLI-heavy changes:

```bash
rtk cargo test -p auralis-cli
```

For Python helper tooling:

```bash
cd tools/pytest
rtk uv sync
rtk uv run pytest
```

For SoX-ng golden work, use a real `sox_ng` binary and make missing oracle
coverage fail release/gnhf gates instead of silently passing:

```bash
export AURALIS_SOX_NG_BIN=${AURALIS_SOX_NG_BIN:-sox_ng}
rtk cargo test --workspace --all-features golden
cd tools/pytest
rtk uv run pytest -m golden
```

For benchmark work, never overwrite `target/benchmarks/sox_ng`; write new runs
to a fresh output directory.

---

## Rust Layout

No Rust source file should exceed 1,000 lines after a change. If a feature would
push a file past that limit, split by functional ownership before finishing the
task.

Run the checked-in guard when a change adds or moves Rust source:

```bash
rtk python3 tools/check_rust_source_lines.py
```

---

## Git Policy

Each successful development iteration should create one focused commit. The
working tree must be clean before starting another iteration.

Commit messages must include:

```text
Co-authored-by: Codex <noreply@openai.com>
```

Do not commit generated large audio artifacts unless they are explicitly part
of a small deterministic corpus.

---

## Project Definition Of Done

Auralis is not done when it has many effects. A milestone is done when the
implemented subset is reliable, documented, deterministic, test-covered,
modular, benchmarkable, and usable from both Rust and the command line.
