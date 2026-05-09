# Auralis Development Guide

This document defines the development process for Auralis. It is intentionally strict because the project aims to become a reliability-oriented audio DSP foundation, not a loose collection of effects.

The core rule:

> Implement one feature at a time. Finish its tests, documentation, examples, and SoX-ng comparison where applicable before starting the next feature.

---

## gnhf stop condition

Use this as the one-sentence stop condition for autonomous development:

> Stop when the next unchecked feature in `DEVELOPMENT.md` is implemented as one small committed change, with formatting, clippy, Rust tests, doc tests, uv-based Python tests, SoX-ng golden tests where applicable, and README/development documentation all passing and updated; do not start the following feature in the same iteration.

Suggested `gnhf` objective:

```bash
gnhf "Implement exactly the next unchecked Auralis feature in DEVELOPMENT.md. Do not skip ahead. Do not start another feature. Add complete tests first or alongside the implementation. The iteration is successful only if cargo fmt, clippy, cargo test, cargo doc tests, and uv pytest pass, and if the change is committed with a concise message."
```

For parallel work, use worktrees only when features are independent. Avoid parallel work on the same module until the core API is stable.

---

## Development invariants

These rules apply to every feature.

### Feature isolation

Each development iteration should modify one conceptual feature only.

Examples of valid single features:

- create workspace skeleton
- implement `AudioSpec`
- implement PCM16 WAV decode
- implement `gain` scalar kernel
- add SoX-ng golden test for `gain`
- add CLI `inspect`

Examples of invalid mixed features:

- implement WAV, gain, and trim together
- add SIMD before scalar tests exist
- add Python bindings before Rust API is stable
- add filter effects before basic buffer and chunk invariance are tested

### Test-first or test-locked development

A feature may be implemented before its tests only if the same commit includes the tests. A feature is not complete without tests.

Required test classes depend on feature type:

| Feature type | Required tests |
|---|---|
| Data type / API | unit tests, doc tests |
| WAV I/O | unit tests, integration tests, round-trip tests, unsupported-format tests |
| Simple DSP | analytical tests, property tests, chunk invariance, SoX-ng golden tests if SoX-ng has equivalent effect |
| Stateful DSP | analytical/structural tests, chunk invariance, edge-case tests, golden tests |
| SIMD | scalar-vs-SIMD differential tests, tail-length tests, benchmarks |
| CLI | command tests, error tests, library-equivalent behavior tests |
| Python test harness | uv-based pytest tests |

### No hidden behavior

If a behavior matters, it must be represented in one of:

- type definition
- effect config
- documented default
- test case
- pipeline manifest

Do not rely on undocumented implicit defaults.

### No panics in library API

The library should return typed errors instead of panicking for user-controlled inputs.

Panics are acceptable only for internal invariant violations and should be rare.

### Documentation is part of completion

Each public API must have documentation before the feature is considered complete.

Minimum documentation:

- purpose
- parameter units
- errors
- determinism
- examples
- numerical behavior if relevant

### Third-party dependency boundaries

Auralis dependency choices are split into three groups:

- add now: `clap`, `clap_complete`, `clap_mangen`, `thiserror`, `miette`,
  `anyhow`, `serde`, `toml`, `serde_json`, `hound`, `tracing`,
  `tracing-subscriber`, and the first test/bench tools
- selected but optional or later: `rten-simd`, `realfft`, `rustfft`, `rayon`,
  `bytemuck`, `smallvec`, `pyo3`, `maturin`, and `numpy`
- do not introduce now: `rubato`, `symphonia`, `ndarray` in core public APIs,
  `serde_yaml`, and `tokio`

The rule is strict: public APIs do not depend on concrete implementation
crates. Third-party crates belong at boundary layers, test layers, CLI layers,
or replaceable backend layers.

Per-crate rules:

| Crate | Allowed dependencies now | Must not expose or depend on |
|---|---|---|
| `auralis-core` | `thiserror`; optional `serde` only when serialization is a feature | `hound`, `clap`, `rten-simd`, `pyo3`, `ndarray`, `rayon` |
| `auralis-codec` / `auralis-wav` | `thiserror`, `hound` where WAV is implemented | `hound` types in public core APIs |
| `auralis-dsp` | `thiserror`; later optional `rten-simd` and `bytemuck` | backend crate names in public effect APIs |
| `auralis-effects` | `thiserror`, `serde` | CLI parser or codec implementation types |
| `auralis-pipeline` | `thiserror`, `serde`, `tracing` | CLI diagnostics or concrete codec internals |
| `auralis-cli` | `clap`, `clap_complete`, `miette`, `tracing`, `tracing-subscriber`, `serde`, `toml`; `clap_mangen` for generated docs | library APIs returning `anyhow::Result<T>` |
| CLI integration tests | `assert_cmd`, `predicates`, `tempfile`, `insta` | test dependencies becoming runtime dependencies |
| `auralis-testkit` | `approx`, `proptest`, `tempfile`, `serde`, `serde_json`, `criterion`; later `realfft`, `rustfft` | dependencies leaking back into production API requirements |

Error handling rules:

- library crates expose typed errors with `thiserror`
- CLI converts errors into `miette` diagnostics
- `anyhow` is limited to binaries, tests, examples, and short-lived glue code
- no library public API returns `anyhow::Result<T>`

Format and configuration rules:

- pipeline manifests use TOML through `serde` and `toml`
- machine-readable test reports use JSON through `serde_json`
- do not add YAML support
- WAV uses `hound` behind Auralis codec traits; tests compare decoded PCM unless
  the test is explicitly about container serialization

Optimization rules:

- scalar DSP is the reference implementation
- SIMD starts only after scalar tests, chunk invariance, and scalar-vs-SIMD
  differential tests exist
- Rayon, bytemuck, and smallvec are optional optimizations, not default design
  assumptions

---

## Required commands before every commit

Run from repository root unless stated otherwise.

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --doc --workspace
```

If Python tests exist for the current milestone:

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
uv run pytest -m golden
```

A commit that cannot pass the required checks must not be accepted as a completed feature.

---

## Baseline reference: SoX-ng

`sox_ng` is the initial behavioral oracle for comparable effects. It is not the architectural oracle.

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

`-R` is used for repeatability. `-D` disables automatic dither so normal DSP tests are not polluted by noise.

For sample comparison, prefer decoded samples over full container bytes.

---

## Python and uv policy

Python is used for numerical tests, golden comparisons, corpus generation, and failure artifact generation.

All Python work must use `uv`.

Expected structure:

```text
tools/pytest/
├── pyproject.toml
├── uv.lock
├── tests/
│   ├── test_wav_io.py
│   ├── test_gain_golden.py
│   └── test_metrics.py
└── auralis_testkit/
    ├── corpus.py
    ├── metrics.py
    ├── sox_ng.py
    └── wav.py
```

Initial Python dependencies:

```toml
[project]
name = "auralis-pytest"
version = "0.0.0"
requires-python = ">=3.11"
dependencies = [
  "numpy",
  "pytest",
  "scipy",
]
```

Optional later dependencies:

```toml
matplotlib = "for failure plots"
hypothesis = "for Python-side property tests"
```

Do not use system `pip install` in project instructions or tests.

---

## Repository bootstrap plan

### Feature 0.1: workspace skeleton

Status: implemented.

Create:

```text
Cargo.toml
rust-toolchain.toml
crates/auralis-core
crates/auralis-codec
crates/auralis-wav
crates/auralis-dsp
crates/auralis-effects
crates/auralis-simd
crates/auralis-cli
crates/auralis-testkit
tools/pytest
```

Acceptance tests:

- `cargo metadata` succeeds.
- `cargo test --workspace` succeeds.
- `cargo doc --workspace --no-deps` succeeds.
- Empty `uv` project can run `uv run pytest` with one placeholder test.

Completion criteria:

- Workspace builds.
- README and DEVELOPMENT references match actual paths.
- No implementation beyond skeleton.

---

### Feature 0.2: core error and type vocabulary

Status: implemented.

Implement in `auralis-core`:

- `AuralisError`
- `Result<T>`
- `SampleRate`
- `ChannelCount`
- `FrameCount`
- `Hertz`
- `Decibels`
- `TimeSeconds`
- `SampleFormat`
- `AudioSpec`

Acceptance tests:

- Valid and invalid sample rates.
- Valid and invalid channel counts.
- Display/debug formatting.
- Doc examples compile.

Do not implement WAV or DSP yet.

---

### Feature 0.3: internal audio buffer

Status: implemented.

Implement:

- `AudioBuffer`
- planar `f32` storage
- frame/channel indexing
- safe channel views
- mutable channel views
- zero initialization
- shape validation

Acceptance tests:

- mono buffer layout
- stereo buffer layout
- channel view mutation
- invalid data length rejection
- zero-length buffer handling
- doc examples compile

Do not implement file I/O yet.

---

## WAV milestone

### Feature 1.1: WAV codec trait boundary

Status: implemented.

Implement in `auralis-codec`:

- `AudioReader` trait
- `AudioWriter` trait
- `CodecKind`
- `UnsupportedFormat` errors
- codec capability description

Acceptance tests:

- placeholder formats return unsupported errors.
- WAV is represented as supported only when `auralis-wav` is enabled.
- public documentation describes initial WAV-only scope.

Do not decode WAV yet.

---

### Feature 1.2: PCM16 WAV decode

Status: implemented.

Implement `auralis-wav` PCM16 WAV reading into planar `f32`.

Acceptance tests:

- decode mono PCM16.
- decode stereo PCM16.
- verify scaling from `i16` to `f32`.
- preserve sample rate and channel count.
- reject unsupported bit depths with typed errors.
- reject malformed WAV gracefully.
- compare decoded samples against Python reference for small fixtures.

SoX-ng comparison:

- For at least one mono and one stereo fixture, compare decoded sample values with `sox_ng` converted to raw float.

Do not implement writing yet.

---

### Feature 1.3: PCM16 WAV encode

Implement planar `f32` to PCM16 WAV writing.

Acceptance tests:

- encode mono PCM16.
- encode stereo PCM16.
- clipping behavior documented and tested.
- round-trip generated signals through Auralis read/write.
- compare sample output with SoX-ng for controlled input where applicable.

Do not implement effects yet.

---

### Feature 1.4: CLI `inspect`

Implement:

```bash
auralis inspect input.wav
```

Output should include:

- format
- sample rate
- channels
- sample format
- duration frames
- duration seconds

Acceptance tests:

- CLI returns expected fields for fixtures.
- invalid path returns non-zero status and clear error.
- unsupported format returns non-zero status and clear error.

Do not implement transform CLI yet.

---

### Feature 1.5: CLI copy pipeline

Implement:

```bash
auralis run input.wav output.wav
```

This must decode through internal planar `f32` and re-encode, not simply copy bytes.

Acceptance tests:

- mono copy sample equivalence within PCM16 quantization tolerance.
- stereo copy sample equivalence within PCM16 quantization tolerance.
- metadata fields are sane.
- unsupported files fail clearly.

Completion of WAV milestone:

- WAV read/write path is stable enough for effect tests.

---

## Basic DSP milestone

### Feature 2.1: scalar `gain` kernel

Implement in `auralis-dsp`:

```rust
pub fn gain_in_place(samples: &mut [f32], db: Decibels)
```

or equivalent buffer-level API.

Acceptance tests:

- `0 dB` identity.
- `-6 dB` equals multiply by `10^(-6 / 20)` within strict tolerance.
- `+6 dB` equals multiply by `10^(6 / 20)` within strict tolerance.
- empty slice.
- one sample.
- odd lengths.
- full-scale input behavior documented.
- no NaN for finite input.

Do not add CLI effect yet.

---

### Feature 2.2: `Gain` effect processor

Implement typed effect:

```rust
pub struct Gain {
    pub db: Decibels,
}
```

Acceptance tests:

- effect output matches scalar kernel.
- whole-buffer and chunked-buffer processing match.
- effect documentation includes examples.

Do not add CLI effect yet.

---

### Feature 2.3: library chain API for gain

Implement chainable call:

```rust
AudioFile::open_wav("input.wav")?
    .into_pipeline()
    .gain_db(-3.0)
    .write_wav("output.wav")?;
```

Acceptance tests:

- doc example compiles.
- chain call output matches direct effect execution.
- errors propagate without panic.

Do not add more effects yet.

---

### Feature 2.4: CLI `gain`

Implement:

```bash
auralis run input.wav output.wav --gain-db -3
```

Acceptance tests:

- CLI output matches library output.
- CLI output matches SoX-ng golden within threshold.
- invalid gain argument fails clearly.
- help text documents units.

Python tests:

- generate fixture with numpy.
- run Auralis CLI.
- run SoX-ng CLI.
- compare decoded samples.

---

### Feature 2.5: `trim`

Implement typed trim by frame range and seconds range.

Acceptance tests:

- exact frame range.
- seconds-to-frame conversion behavior documented.
- full-range trim identity.
- empty trim result.
- mono and stereo frame grouping preserved.
- CLI and library behavior match.
- SoX-ng golden comparison for representative cases.

Do not implement `pad` until `trim` is complete.

---

### Feature 2.6: `pad`

Implement zero padding at start and end.

Acceptance tests:

- zero pad identity.
- start pad exact zeros.
- end pad exact zeros.
- mono/stereo shape preserved.
- CLI and library behavior match.
- SoX-ng golden comparison where command semantics align.

---

### Feature 2.7: `reverse`

Implement frame-level reverse.

Acceptance tests:

- mono exact reverse.
- stereo frame-level reverse, not sample-level channel swap.
- reverse twice is identity.
- zero-length and one-frame buffers.
- CLI and library behavior match.
- SoX-ng golden comparison.

---

### Feature 2.8: `dcshift`

Implement constant DC offset.

Acceptance tests:

- zero shift identity.
- positive and negative shifts.
- clipping behavior documented.
- no NaN for finite input.
- CLI and library behavior match.
- SoX-ng golden comparison if command semantics align.

---

### Feature 2.9: `fade`

Implement simple linear fade-in and fade-out.

Acceptance tests:

- envelope coefficients are correct.
- zero-duration fade identity or documented error.
- fade-in only.
- fade-out only.
- mono/stereo behavior.
- chunk invariance.
- CLI and library behavior match.

Only after these basic effects are complete should filters begin.

---

## Test infrastructure milestone

Some test infrastructure can be implemented alongside early features, but each piece must remain small.

### Feature 3.1: Rust metrics module

Implement:

- `max_abs_error`
- `rms_error`
- `snr_db`
- `peak`
- `dc_offset`

Acceptance tests:

- exact known vectors.
- zero-reference behavior documented.
- NaN behavior documented.

---

### Feature 3.2: Python testkit under uv

Create `tools/pytest` with:

- `pyproject.toml`
- `auralis_testkit/corpus.py`
- `auralis_testkit/metrics.py`
- `auralis_testkit/sox_ng.py`
- `tests/test_smoke.py`

Acceptance tests:

```bash
cd tools/pytest
uv sync
uv run pytest
```

---

### Feature 3.3: golden manifest format

Define a manifest format for golden tests:

```toml
[id.gain_minus_3_mono]
input = "sine_48k_mono.wav"
auralis = ["gain", "-3"]
sox_ng = ["gain", "-3"]
max_abs = 1e-4
rms = 1e-6
snr_db = 90.0
```

Acceptance tests:

- manifest parse.
- invalid manifest rejected.
- command rendering is deterministic.

---

## SIMD milestone

SIMD must wait until scalar correctness is stable for at least the first basic effects.

### Feature 4.1: SIMD backend trait

Define internal kernel trait without exposing `rten-simd` publicly.

Example:

```rust
pub trait SampleKernelBackend {
    fn gain_in_place(samples: &mut [f32], gain: f32);
    fn mix2(out: &mut [f32], a: &[f32], b: &[f32], wa: f32, wb: f32);
}
```

Acceptance tests:

- scalar backend implements trait.
- dispatch path can force scalar.
- public API unchanged.

Do not add `rten-simd` implementation yet.

---

### Feature 4.2: `rten-simd` gain backend

Implement SIMD gain backend.

Acceptance tests:

- scalar vs SIMD differential tests.
- empty slice.
- one sample.
- tail lengths around vector width.
- seeded random buffers.
- near-clipping values.
- backend can be forced in tests.

Benchmark:

- compare scalar vs SIMD on representative buffer sizes.

Do not SIMD-optimize other kernels yet.

---

### Feature 4.3: SIMD sample conversion

Implement:

- `i16_to_f32`
- `f32_to_i16`

Acceptance tests:

- exact known conversions.
- clipping and rounding documented.
- scalar vs SIMD differential.
- WAV encode/decode tests still pass.

---

## Filter milestone

Filters begin only after basic effects and testkit are stable.

### Feature 5.1: biquad primitive

Implement scalar biquad with explicit coefficient structure.

Acceptance tests:

- known coefficient cases.
- impulse response.
- silence preservation.
- finite input produces finite output.
- chunk invariance.

Do not implement lowpass/highpass user effects yet.

---

### Feature 5.2: lowpass effect

Implement typed lowpass effect using biquad or documented design.

Acceptance tests:

- analytical frequency response.
- impulse response sanity.
- chunk invariance.
- SoX-ng golden comparison with documented tolerance.
- CLI and library behavior match.

---

### Feature 5.3: highpass effect

Same structure as lowpass.

Acceptance tests:

- DC rejection.
- frequency response.
- chunk invariance.
- SoX-ng golden comparison.

---

## Resampler milestone

Do not start the resampler until WAV, basic effects, metrics, golden tests, and chunk invariance infrastructure are mature.

### Feature 6.1: resampler specification

Before implementation, write the spec:

- supported ratios
- quality levels
- phase behavior
- expected latency
- output length formula
- alias rejection thresholds
- comparison strategy against SoX-ng

Acceptance test:

- spec document exists.
- tests are scaffolded but skipped with explicit reason.

---

### Feature 6.2+: implementation steps

Implement only after the spec is reviewed:

1. trivial same-rate identity
2. simple linear resampler for baseline
3. higher-quality polyphase resampler
4. SoX-ng golden comparisons
5. analytical alias tests
6. SIMD inner loop after scalar correctness

---

## Future milestones

These are placeholders, not immediate work:

- delay
- echo
- reverb
- compand
- silence detection
- rate quality modes
- FLAC support
- AIFF support
- raw PCM support
- Python package via PyO3
- NumPy-compatible buffer interface
- stable public crate release

Do not start these until earlier milestones have passed.

---

## Acceptance checklist template

Every feature should add or update a checklist like this in the relevant issue, PR, or commit note:

```text
Feature: <name>

Implementation:
[ ] Typed API added
[ ] CLI integration added, if applicable
[ ] Error handling added
[ ] Documentation added
[ ] Examples added

Tests:
[ ] Unit tests
[ ] Doc tests
[ ] Integration tests
[ ] SoX-ng golden tests, if applicable
[ ] Analytical tests, if applicable
[ ] Property/metamorphic tests, if applicable
[ ] Chunk invariance tests, if applicable
[ ] Scalar-vs-SIMD tests, if applicable
[ ] Python uv pytest tests, if applicable

Quality gate:
[ ] cargo fmt --all --check
[ ] cargo clippy --workspace --all-targets --all-features -- -D warnings
[ ] cargo test --workspace --all-features
[ ] cargo test --doc --workspace
[ ] uv run pytest, if Python tests exist
[ ] cargo bench, if performance-sensitive
```

A feature is incomplete until all applicable items are checked.

---

## Commit policy for gnhf-driven work

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
- Do not commit generated large audio artifacts unless they are explicitly part of the small deterministic corpus.
- Prefer generated fixtures over large binary fixtures.

---

## Failure artifact policy

When a numerical test fails, it should report enough detail for an agent or human to debug.

Required failure data:

- case ID
- input corpus ID
- command or API call
- metric name
- expected value
- actual value
- maximum offending index if applicable
- output length
- backend used
- sample rate
- channel count

Optional artifacts:

- `.json` metric report
- waveform diff `.npy`
- spectrum diff `.npy`
- plot image generated only for manual inspection

Do not rely on visual plots as the pass/fail mechanism.

---

## API taste rules

Auralis APIs should be explicit, typed, and calm.

Prefer:

```rust
pipeline.gain_db(-3.0)
pipeline.trim_seconds(0.0..10.0)
pipeline.write_wav("out.wav")
```

Avoid:

```rust
pipeline.effect("gain -3")
pipeline.do_magic("trim", vec!["0", "10"])
```

String-based APIs may exist for CLI compatibility, but typed APIs are the primary library interface.

---

## When to add SIMD

Only add SIMD when all are true:

1. scalar implementation exists,
2. scalar tests are complete,
3. profiler or benchmark shows the kernel matters,
4. kernel is suitable for SIMD,
5. scalar-vs-SIMD tests can be written,
6. backend selection can be controlled in tests.

Do not add SIMD because it looks elegant.

`rten-simd` is the selected portable SIMD direction, but it remains optional and
must be hidden behind Auralis-owned backend traits. Do not expose `rten-simd`
types from public library APIs.

---

## When to add Python bindings

Only add PyO3 bindings when all are true:

1. Rust library API is stable enough to expose,
2. WAV pipeline is tested,
3. basic effects are tested,
4. error model is stable,
5. buffer model is stable,
6. Python package behavior can be documented clearly.

Before then, Python is only a test harness.

---

## Project definition of done

Auralis is not “done” when it has many effects. It is done for a milestone when the implemented subset is reliable, documented, deterministic, test-covered, and pleasant to use from both CLI and Rust.
