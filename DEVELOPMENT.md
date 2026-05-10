# Auralis Development Guide

This file is the stable development index for Auralis. Keep it at the repository
root. Detailed milestone plans live under [`doc/development/`](doc/development/).

The core rule:

> Implement one feature at a time. Finish its tests, documentation, examples,
> and SoX-ng comparison where applicable before starting the next feature.

---

## Current priority

The current development queue is blocked on planning and test-gate debt, not on
the old `18d002f` dither-order correction. Recent gnhf work reached specialized
effects planning and exposed that `dolbyb` is not ready to proceed as the next
implementation target without rebalancing the roadmap and golden-test gates.

The current order is:

1. Finish the plan reshuffle for the oversized effects plan and README status
   surface: detailed effect status belongs in
   [`doc/development/06-effects/`](doc/development/06-effects/), while README
   remains a short user-facing entry point.
2. Strengthen the test gates before more effect implementation: missing
   `sox_ng` now fails explicit release/gnhf golden checks through the Rust
   `golden` test filter and pytest `-m golden` gate; complex pipeline goldens
   now include the first multi-input combine-plus-effects case and the first
   output-channel policy plus user-effects case, while boundary stress
   coverage remains pending, and coverage artifacts must still be produced by
   the applicable gate.
3. Continue Rust-first migration of critical SoX-ng golden and complex pipeline
   coverage. Python/pytest may remain as helper tooling, but key behavior gates
   should move into Rust/testkit where feasible.
4. Resume
   [`6.9 specialized and integration effects`](doc/development/06-effects/06-9-specialized-and-integration-effects.md)
   only after the above planning and test-gate corrections are in place.
5. After the 6.9 specialized-effect classification pass, run the
   [`7.x reusable DSP primitives roadmap`](doc/development/07-dsp-primitives.md)
   before adding more effect families, so shared algorithms are audited before
   they continue accumulating in effect-local modules.

The previous Feature 5.5.4 entry is retained only as a historical correction in
the 5.x plan. It must not be selected as the next gnhf implementation target.

---

## gnhf Stop Condition

Use this as the stop condition for autonomous development:

> Stop only when every feature listed in this file and the linked
> `doc/development/*.md` and `doc/development/06-effects/*.md` files is
> implemented, tested, documented, committed, and pushed. If a feature cannot be
> completed safely, stop after recording the blocker in the relevant
> development-plan file. For each loop iteration, implement exactly the next
> unchecked leaf feature, keep it to one focused commit, run formatting, clippy,
> Rust tests, doc tests, required SoX-ng golden tests, and optional/helper
> uv-based Python tests where they exist. Missing `sox_ng` must fail
> release/gnhf golden gates instead of silently skipping coverage. README should
> summarize current status only; detailed implementation status belongs in
> development-plan files.

Suggested `gnhf` objective:

```bash
gnhf --current-branch --push "Repeatedly implement Auralis features from DEVELOPMENT.md, doc/development/*.md, and doc/development/06-effects/*.md in order. In each iteration, implement exactly the next unchecked leaf feature and do not skip ahead. Add complete tests first or alongside the implementation. A feature is successful only if cargo fmt, clippy, cargo test, cargo doc tests, required SoX-ng golden tests, complex pipeline golden coverage where applicable, coverage report artifact generation where applicable, and optional/helper uv pytest checks pass. Missing sox_ng must fail release/gnhf golden checks. Keep README as a compact index/status entry and put detailed effect status in development-plan files. Continue with the next unchecked feature after each successful commit. The source code of sox_ng is in /root/code/sox-rs/sox_ng" --stop-when "Stop only when every feature listed in DEVELOPMENT.md, doc/development/*.md, and doc/development/06-effects/*.md is implemented, tested, documented, committed, and pushed; if a feature cannot be completed safely, stop after recording the blocker."
```

For parallel work, use worktrees only when features are independent. Avoid
parallel work on the same module until the core API is stable.

---

## Development Plan Index

| Track | File | Scope |
|---|---|---|
| 0.x | [`00-bootstrap.md`](doc/development/00-bootstrap.md) | workspace skeleton, core vocabulary, internal buffer |
| 1.x | [`01-wav.md`](doc/development/01-wav.md) | WAV trait boundary, PCM16 decode/encode, inspect, copy pipeline |
| 2.x | [`02-basic-dsp.md`](doc/development/02-basic-dsp.md) | gain, trim, pad, reverse, dcshift, fade |
| 3.x | [`03-test-infrastructure.md`](doc/development/03-test-infrastructure.md) | metrics, uv pytest harness, golden manifests, SoX-ng coverage rules |
| 4.x | [`04-simd.md`](doc/development/04-simd.md) | backend dispatch, sample conversion, SIMD retrofits |
| 5.x | [`05-pipeline-parity.md`](doc/development/05-pipeline-parity.md) | chains, effects files, input/output policies, modularization debt, layered-test debt |
| 6.x | [`06-effect-coverage.md`](doc/development/06-effect-coverage.md) | remaining SoX-ng effect coverage |
| 7.x | [`07-dsp-primitives.md`](doc/development/07-dsp-primitives.md) | reusable DSP primitive audit and extraction roadmap |
| 8.x | [`08-format-support.md`](doc/development/08-format-support.md) | richer WAV and additional formats |
| 9.x | [`09-python-package.md`](doc/development/09-python-package.md) | future PyO3/maturin packaging |

Milestone headings group work only. A `Feature x.y.z` item is one gnhf
iteration and one focused commit unless the plan explicitly says it is a
historical note or a non-implementation placeholder.

---

## Development Invariants

### Feature isolation

Each development iteration should modify one conceptual feature only.

Valid single features include:

- create workspace skeleton
- implement `AudioSpec`
- implement PCM16 WAV decode
- implement a scalar DSP kernel
- add SoX-ng golden tests for one effect family
- add one CLI command surface

Invalid mixed features include:

- implement WAV, gain, and trim together
- add SIMD before scalar tests exist
- add Python bindings before the Rust API is stable
- add filter effects before basic buffer and chunk invariance are tested

### Test-first or test-locked development

A feature may be implemented before its tests only if the same commit includes
the tests. A feature is not complete without its full applicable test set.

Required test classes depend on feature type:

| Feature type | Required tests |
|---|---|
| Data type / API | unit tests, doc tests |
| WAV I/O | unit tests, integration tests, round-trip tests, unsupported-format tests |
| Simple DSP | analytical tests, property tests, chunk invariance, SoX-ng golden tests if SoX-ng has equivalent behavior |
| Stateful DSP | analytical/structural tests, chunk invariance, edge-case tests, golden tests |
| SIMD | scalar-vs-SIMD differential tests, tail-length tests, benchmarks where performance-sensitive |
| CLI | command tests, error tests, library-equivalent behavior tests |
| Python helper/testkit tooling | uv-based smoke/helper tests, with authoritative gates named in Rust/testkit where applicable |

### L0-L7 test contract

[`doc/testing.md`](doc/testing.md) is the user-facing authoritative layered
test contract. [`03-test-infrastructure.md`](doc/development/03-test-infrastructure.md)
is the development-plan authority for implementing and evolving that contract.
README may summarize L0-L7, but it must not carry detailed status or override
those files.

- L0 deterministic corpus
- L1 WAV I/O correctness
- L2 SoX-ng golden regression
- L3 analytical DSP tests
- L4 property and metamorphic tests
- L5 chunk invariance
- L6 scalar-vs-SIMD differential tests
- L7 fuzzing, sanitizers, and coverage

Every new feature must state which layers apply and must either implement them
or document a narrow N/A reason. The current gaps are tracked in
[`03-test-infrastructure.md`](doc/development/03-test-infrastructure.md),
[`05-pipeline-parity.md`](doc/development/05-pipeline-parity.md), and the
layered coverage matrix.

### No hidden behavior

If a behavior matters, it must be represented in one of:

- type definition
- effect config
- documented default
- test case
- pipeline manifest

Do not rely on undocumented implicit defaults.

### No panics in library API

The library should return typed errors instead of panicking for user-controlled
inputs. Panics are acceptable only for internal invariant violations and should
be rare.

### Documentation is part of completion

Each public API must have documentation before the feature is complete.

Minimum documentation:

- purpose
- parameter units
- errors
- determinism
- examples
- numerical behavior if relevant

### Source file size and module boundaries

Large `lib.rs` implementation warehouses are not acceptable. `lib.rs` files
should primarily define crate-level docs, module declarations, and re-exports.

Policy:

- no Rust source file should exceed 1,000 lines after the modularization pass;
- no implementation-heavy `lib.rs` should remain near the limit;
- each crate should be split by functional ownership, not by arbitrary line
  count;
- tests may be moved into `tests/` modules or focused `#[cfg(test)]` module
  files when they dominate source size;
- future features must not reintroduce thousand-line single files.

Run the checked-in guard before completing feature work that adds or moves Rust
source:

```bash
python3 tools/check_rust_source_lines.py
```

Known offenders at the time this plan was written:

| File | Approximate lines | Required action |
|---|---:|---|
| `crates/auralis/src/lib.rs` | 4183 | split facade, pipeline, combiners, output policies, and tests |
| `crates/auralis-simd/src/lib.rs` | 2466 | split backend selection, conversion, kernels, and conformance tests |
| `crates/auralis-effects/src/lib.rs` | 1103 | split effect implementations and effect tests |
| `crates/auralis-wav/src/lib.rs` | 1079 | split reader, writer, format validation, and tests |

The exact Feature 5.6.1 file-size audit and ownership map is checked in at
[`doc/development/05-source-module-map.md`](doc/development/05-source-module-map.md).
That audit supersedes the approximate counts above and identifies additional
oversized test/testkit files that must be split before the no-thousand-line-file
policy can be enforced. Feature 5.6.6 completed those remaining splits and
added the guard command above.

The source modularization plan is Feature 5.6.x.

### Third-party dependency boundaries

Auralis dependency choices are split into three groups:

- add now: `clap`, `clap_complete`, `clap_mangen`, `thiserror`, `miette`,
  `anyhow`, `serde`, `toml`, `serde_json`, `hound`, `rustfft`, `tracing`,
  `tracing-subscriber`, and the first test/bench tools
- selected but optional or later: `rten-simd`, `realfft`, `rayon`,
  `bytemuck`, `smallvec`, `pyo3`, `maturin`, and `numpy`
- do not introduce now: `rubato`, `symphonia`, `ndarray` in core public APIs,
  `serde_yaml`, and `tokio`

The rule is strict: public APIs do not depend on concrete implementation
crates. Third-party crates belong at boundary layers, test layers, CLI layers,
or replaceable backend layers.

Library crates expose typed errors with `thiserror`. CLI code converts errors
into `miette` diagnostics. `anyhow` is limited to binaries, tests, examples,
and short-lived glue code. No library public API returns `anyhow::Result<T>`.

Pipeline manifests use TOML. Machine-readable test reports use JSON. Do not add
YAML support.

---

## Required Commands Before Every Commit

Run from repository root unless stated otherwise.

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --doc --workspace
```

If Python helper tests exist for the current milestone:

```bash
cd tools/pytest
uv sync
uv run pytest
```

If the feature touches performance-sensitive kernels:

```bash
cargo bench
```

If the feature uses SoX-ng golden tests:

```bash
export AURALIS_SOX_NG_BIN=${AURALIS_SOX_NG_BIN:-sox_ng}
cargo test --workspace --all-features golden
cd tools/pytest
uv run pytest -m golden  # optional/helper layer when Python goldens still exist
```

A commit that cannot pass the required checks must not be accepted as a
completed feature.

---

## Baseline Reference: SoX-ng

`sox_ng` is the initial behavioral oracle for comparable effects. It is not the
architectural oracle.

Rules:

- Use SoX-ng to define golden output for implemented effects.
- Do not copy SoX-ng internals blindly.
- Prefer analytical tests over golden tests when the effect is mathematically simple.
- Use both golden and analytical tests when possible.
- Always record the SoX-ng command used to generate or compare output.

Recommended invocation style:

```bash
sox_ng -R -D input.wav output.wav gain -3
```

`-R` is used for repeatability. `-D` disables automatic dither so normal DSP
tests are not polluted by noise. Dither tests are the exception and must use
their own explicit random/seed policy.

For sample comparison, prefer decoded samples over full container bytes.

---

## Python and uv Policy

Python is retained as an optional/helper layer for numerical experiments, golden
comparison utilities, corpus generation, smoke checks, and failure artifact
generation. It is not required for all feature behavior, and feature acceptance
must not depend on duplicating every Rust behavior in pytest.

Critical SoX-ng golden coverage and complex pipeline behavior should be
implemented in Rust/testkit where feasible, especially for release and gnhf
gates. Existing duplicate Python goldens should either migrate into Rust/testkit
or be downgraded to smoke/helper coverage with the authoritative gate named in
the Rust test plan. All remaining Python work must use `uv`.

Expected structure:

```text
tools/pytest/
├── pyproject.toml
├── uv.lock
├── tests/
└── auralis_testkit/
```

Do not use system `pip install` in project instructions or tests.

---

## Acceptance Checklist Template

Every feature should add or update a checklist like this in the relevant issue,
PR, or commit note:

```text
Feature: <name>

Implementation:
[ ] SoX-ng coverage entry updated, if this is an effect or pipeline feature
[ ] Typed API added
[ ] CLI integration added, if applicable
[ ] Scalar reference path added, if this processes samples
[ ] SIMD backend added, or SIMD N/A reason documented
[ ] Backend selection can be forced in tests, if this processes samples
[ ] Error handling added
[ ] Documentation added
[ ] Examples added
[ ] Source files stay below the 1,000-line policy

Tests:
[ ] L0 deterministic corpus case added or reused, if applicable
[ ] L1 WAV I/O tests, if applicable
[ ] L2 SoX-ng golden tests, if applicable
[ ] Complex pipeline golden test added or updated, if applicable
[ ] L3 analytical tests, if applicable
[ ] L4 property/metamorphic tests, if applicable
[ ] L5 chunk invariance tests, if applicable
[ ] L6 scalar-vs-SIMD tests, or SIMD N/A reason checked
[ ] L7 fuzz/sanitizer/coverage target added or explicitly not applicable
[ ] Layered coverage matrix row added or updated, if behavior or test surface changes
[ ] Coverage report artifact generated or explicitly not applicable
[ ] Rust-first golden coverage used where feasible
[ ] Unit tests
[ ] Doc tests
[ ] Integration tests
[ ] CLI and typed API equivalence tests, if applicable
[ ] Python uv pytest smoke/helper tests, if applicable

Quality gate:
[ ] cargo fmt --all --check
[ ] cargo clippy --workspace --all-targets --all-features -- -D warnings
[ ] cargo test --workspace --all-features
[ ] cargo test --doc --workspace
[ ] uv run pytest, if Python helper tests exist
[ ] SoX-ng golden test command, if golden manifests exist; missing sox_ng fails release/gnhf gates
[ ] Complex pipeline golden command, if pipeline manifests exist
[ ] Coverage report artifact command, if coverage is applicable
[ ] cargo bench, if performance-sensitive
```

A feature is incomplete until all applicable items are checked.

---

## Commit Policy for gnhf-driven Work

Each successful iteration should create one commit.

Commit message style:

```text
feat(core): add AudioSpec and unit types
test(wav): add PCM16 decode fixtures
feat(dsp): implement scalar gain kernel
```

Rules:

- The working tree must be clean before starting a new gnhf run.
- Each commit must pass the required checks for the feature.
- Failed experiments should be reverted or clearly contained before commit.
- Do not commit generated large audio artifacts unless they are explicitly part
  of the small deterministic corpus.
- Prefer generated fixtures over large binary fixtures.

---

## Project Definition of Done

Auralis is not done when it has many effects. It is done for a milestone when
the implemented subset is reliable, documented, deterministic, test-covered,
modular, and pleasant to use from both CLI and Rust.
