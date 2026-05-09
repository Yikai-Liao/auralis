# Auralis Development Guide

This document defines the development process for Auralis. It is intentionally strict because the project aims to become a reliability-oriented audio DSP foundation, not a loose collection of effects.

The core rule:

> Implement one feature at a time. Finish its tests, documentation, examples, and SoX-ng comparison where applicable before starting the next feature.

---

## gnhf stop condition

Use this as the stop condition for autonomous development:

> Stop only when every feature listed in `DEVELOPMENT.md` is implemented, tested, documented, committed, and pushed; if a feature cannot be completed safely, stop after recording the blocker. For each loop iteration, implement exactly the next unchecked feature, keep it to one focused commit, run formatting, clippy, Rust tests, doc tests, uv-based Python tests, SoX-ng golden tests where applicable, update README/development documentation, commit, push, then continue to the next unchecked feature.

Suggested `gnhf` objective:

```bash
gnhf --current-branch --push "Repeatedly implement Auralis features from DEVELOPMENT.md in order. In each iteration, implement exactly the next unchecked feature and do not skip ahead. Add complete tests first or alongside the implementation. A feature is successful only if cargo fmt, clippy, cargo test, cargo doc tests, and uv pytest pass; SoX-ng golden tests must pass where applicable; README/development documentation must be updated; and the feature is committed with a concise message before moving to the next feature. Continue with the next unchecked feature after each successful commit. The source code of sox_ng is in /root/code/sox-rs/sox_ng" --stop-when "Stop only when every feature listed in DEVELOPMENT.md is implemented, tested, documented, committed, and pushed; if a feature cannot be completed safely, stop after recording the blocker."
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
| `auralis` | `thiserror`, internal Auralis crates | CLI parser types or concrete backend implementation types beyond facade boundaries |
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
crates/auralis
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

Status: implemented.

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

Status: implemented.

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

Status: implemented.

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

Status: implemented.

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

Status: implemented.

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

Status: implemented.

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

Status: implemented.

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

Status: implemented.

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

Status: implemented.

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

Status: implemented.

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

Status: implemented.

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

Status: implemented.

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

Status: implemented.

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

Status: implemented.

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

Status: implemented.

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

## SoX-ng coverage strategy

From this point forward, prioritize SoX-ng effect and pipeline behavior before
adding more file formats. WAV PCM16 remains the canonical interchange container
for golden tests until the effect surface is broad and stable.

The current SoX-ng effect surface to track is:

```text
allpass band bandpass bandreject bass bend biquad centercut channels chorus
compand contrast dcshift deemph delay dither dolbyb dop downsample earwax echo
echos equalizer fade fir firfit flanger gain highpass hilbert ladspa loudness
lowpass mcompand noiseprof noisered norm oops overdrive pad phaser pitch rate
remix repeat reverb reverse riaa saturation sdm silence sinc softvol speed
splice stat stats stretch swap synth tempo treble tremolo trim upsample vad vol
```

Every new effect feature must add or update a SoX-ng coverage entry with:

- SoX-ng command syntax and supported options.
- Auralis typed API and CLI mapping.
- implemented / partial / blocked status.
- scalar backend status.
- SIMD backend status or an explicit N/A reason.
- golden cases and tolerances.
- known semantic differences from SoX-ng.

Do not claim SoX-ng parity for an effect until its options, edge cases, pipeline
positioning, CLI behavior, and golden tests are covered.

---

## Feature granularity after 3.3

From Feature 3.3 onward, every commit-sized work item must be a leaf feature.
Milestone headings group work only; they are not implementation targets.

Rules:

- A `Feature x.y.z` item is one gnhf iteration and one focused commit.
- A milestone or group such as `6.1` is not directly implementable.
- If a leaf feature grows beyond one effect, one option family, one backend
  kernel, or one pipeline primitive, split it before implementation.
- Each leaf feature must satisfy the acceptance checklist on its own.
- Shared test helpers may be a separate leaf feature when adding them would make
  the effect commit too large.

---

## SIMD foundation milestone

SIMD starts early. Every new sample-processing feature after Feature 3.3 must
ship with both a scalar reference path and a SIMD backend when the core loop is
data-parallel. If SIMD is not applicable, the feature must document why.

### Milestone 4.1: backend contract and dispatch

#### Feature 4.1.1: backend trait skeleton

Status: implemented.

Define internal backend traits without exposing `rten-simd` publicly.

Acceptance tests:

- scalar backend implements the backend trait.
- public library APIs do not expose backend crate types.
- a placeholder SIMD backend can be compiled where the selected backend crate is
  available.

---

#### Feature 4.1.2: deterministic backend selection

Status: implemented.

Implement named backends:

- `scalar`
- `simd`

Acceptance tests:

- tests can force scalar.
- tests can force SIMD when available.
- unsupported SIMD platforms fall back to scalar with a documented reason.
- backend choice does not change public APIs.

---

#### Feature 4.1.3: scalar-vs-SIMD conformance helpers

Status: implemented.

Implement shared test helpers for backend differential tests.

Acceptance tests:

- helpers compare exact output and tolerance-based output.
- helpers report backend name, case ID, first failing index, and error metrics.
- helpers can run the same case under forced scalar and forced SIMD.

---

### Milestone 4.2: SIMD sample conversion

#### Feature 4.2.1: SIMD `i16_to_f32`

Status: implemented.

Implement scalar and SIMD versions of `i16_to_f32`.

Acceptance tests:

- exact known conversions.
- empty, one-sample, odd-length, and vector-tail lengths.
- seeded random buffers across the full PCM16 range.
- scalar-vs-SIMD output equality.
- WAV decode tests pass under forced scalar and forced SIMD.

---

#### Feature 4.2.2: SIMD `f32_to_i16`

Status: implemented.

Implement scalar and SIMD versions of `f32_to_i16`.

Acceptance tests:

- exact known conversions.
- clipping and rounding documented.
- NaN and infinity behavior documented.
- empty, one-sample, odd-length, and vector-tail lengths.
- seeded random buffers including near-clipping values.
- scalar-vs-SIMD output equality or documented one-LSB tolerance.
- WAV encode tests pass under forced scalar and forced SIMD.

---

### Milestone 4.3: SIMD retrofit for implemented basic effects

#### Feature 4.3.1: SIMD `gain` retrofit

Status: implemented.

Add backend-dispatched kernels for implemented `gain`.

Acceptance tests:

- existing analytical and SoX-ng golden tests pass unchanged.
- scalar and SIMD outputs match for deterministic fixtures.
- random buffers cover silence, near-clipping values, NaN, and infinities where
  public behavior is defined.
- CLI output is identical or within documented tolerance under forced scalar and
  forced SIMD.

---

#### Feature 4.3.2: SIMD `dcshift` retrofit

Status: implemented.

Add backend-dispatched kernels for implemented `dcshift`.

Acceptance tests:

- existing analytical and SoX-ng golden tests pass unchanged.
- scalar and SIMD outputs match for deterministic fixtures.
- random buffers cover silence, denormals, near-clipping values, NaN, and
  infinities where public behavior is defined.
- CLI output is identical or within documented tolerance under forced scalar and
  forced SIMD.

---

#### Feature 4.3.3: SIMD linear `fade` retrofit

Status: implemented.

Add backend-dispatched kernels for linear fade envelope multiplication.

Acceptance tests:

- existing analytical and SoX-ng golden tests pass unchanged.
- scalar and SIMD outputs match for deterministic fade-in, fade-out, and
  combined fade fixtures.
- CLI output is identical or within the documented tolerance under forced scalar
  and forced SIMD.

---

## Effect test contract

Every effect feature after Feature 3.3 must include the following test layers.

### Rust unit and property tests

Required for every effect:

- config parsing and validation.
- typed API construction.
- identity parameters, if the effect has an identity case.
- zero-length, one-frame, mono, stereo, and odd frame counts.
- finite input produces documented finite or non-finite output.
- no library panics for user-controlled parameters.
- property or metamorphic tests where useful, such as reverse twice is identity,
  zero gain is identity, and splitting then joining preserves frame order.

### Chunk invariance tests

Each effect must be tested as whole-buffer processing and as chunked streaming.

Required chunk patterns:

- all-at-once.
- one frame at a time.
- powers of two around the internal block size.
- uneven chunks such as 3, 5, 7, 11, 97.
- empty chunks interleaved with non-empty chunks.
- final flush for effects with latency or tail output.

Stateful effects must document expected latency, warm-up behavior, tail length,
and whether chunked output is bit-exact or tolerance-based.

### Scalar-vs-SIMD tests

Required for every effect with a data-parallel kernel:

- force scalar and SIMD backends on the same input.
- compare empty, tiny, vector-width-minus-one, vector-width, vector-width-plus-one,
  and large buffers.
- test aligned and unaligned logical offsets when the implementation supports
  views or slices.
- test seeded random input and structured signals.
- compare direct effect output and full CLI output.

If SIMD is not applicable, the feature must add a short N/A note to the coverage
entry. Examples: command parser only, external plugin host, or algorithm dominated
by serial state transitions with no useful vectorizable inner loop.

### SoX-ng golden tests

Golden tests use SoX-ng as a behavioral oracle, not as an implementation guide.

Required rules:

- Use `sox_ng -R -D` unless the tested behavior is dithering or randomness.
- Record the exact SoX-ng command in the manifest.
- Generate deterministic WAV PCM16 fixtures through Python or Rust testkit.
- Compare decoded samples, not container bytes.
- Check output sample rate, channel count, frame count, peak, RMS error, max
  absolute error, and SNR where applicable.
- Store tolerances per effect and per option family.
- Save JSON failure reports with the fields listed in the failure artifact policy.

Required fixture families:

- silence.
- impulse and step.
- full-scale and near-full-scale sine.
- swept sine.
- multi-tone signal.
- deterministic white noise.
- stereo phase and channel-identification fixtures.
- short files shorter than filter windows or delay lines.

Tolerance guidance:

- exact editing effects should be sample-exact after PCM quantization.
- simple gain and offset effects should be within PCM16 quantization tolerance.
- IIR/FIR filters should use max error, RMS error, and spectral checks.
- time stretching, pitch, reverb, noise reduction, and dynamics may need broader
  tolerances, but the tolerance must be justified in the manifest.

### CLI and pipeline equivalence tests

Every effect must prove that:

- typed Rust API output matches CLI output.
- single-effect CLI output matches the same effect inside a multi-effect pipeline.
- manifest command rendering is deterministic.
- invalid arguments fail with clear diagnostics and no partial output unless the
  behavior is explicitly documented.

---

## Pipeline parity milestone

Pipeline behavior is now higher priority than additional file formats because
many SoX-ng effects only make sense inside chains.

### Milestone 5.1: effect command model

#### Feature 5.1.1: effect registry and name resolution

Status: implemented.

Implement a registry for supported effect names and aliases.

Acceptance tests:

- supported names resolve to typed effect descriptors.
- unknown names fail with suggestions.
- unsupported SoX-ng effects fail with a message that names the missing coverage.
- existing typed APIs remain unchanged.

---

#### Feature 5.1.2: command parser for implemented effects

Status: implemented.

Implement an internal command model that can represent SoX-ng-style effect
invocations while keeping typed APIs primary.

Acceptance tests:

- parse implemented effect names and options into typed configs.
- reject unsupported options with a diagnostic that names the effect and option.
- no stringly typed effect config leaks into public library APIs.
- preserve typed API behavior for existing effects.

---

#### Feature 5.1.3: deterministic command rendering

Implement deterministic rendering for command manifests and failure reports.

Acceptance tests:

- equivalent command values render identically.
- quoting and escaping are deterministic.
- golden manifest command rendering is stable across runs.

---

### Milestone 5.2: multi-effect chains

#### Feature 5.2.1: in-memory sequential chain

Implement sequential chains in the library.

Acceptance tests:

- multiple effects execute in user-specified order.
- chain output matches repeated direct library calls.
- failures report which effect and option failed.
- chunk invariance holds across the full chain.
- scalar and SIMD backends can be forced for the full chain.

---

#### Feature 5.2.2: CLI sequential chain syntax

Expose multi-effect chains in `auralis run`.

Acceptance tests:

- CLI order matches user-specified order.
- CLI output matches in-memory chain output.
- invalid chain syntax reports the failing effect and argument.
- existing single-effect CLI behavior remains compatible.

---

#### Feature 5.2.3: SoX-ng golden tests for chains

Add golden manifest cases for representative chains.

Acceptance tests:

- at least one editing chain, one level chain, and one filter chain.
- Auralis and SoX-ng command lines are recorded.
- decoded samples and metadata are compared with documented tolerances.

---

### Milestone 5.3: effects files and chain boundaries

#### Feature 5.3.1: effects file parser

Implement a parser for SoX-ng-inspired effects files.

Acceptance tests:

- read effects from a text file.
- ignore blank lines and documented comments.
- reject malformed files with line and column diagnostics.
- parsed effects match equivalent CLI args.

---

#### Feature 5.3.2: effects file CLI integration

Wire effects files into the CLI.

Acceptance tests:

- CLI accepts an effects file.
- effects file output matches equivalent CLI args.
- missing files and unreadable files fail clearly.
- golden tests cover the same chain from CLI args and effects file.

---

#### Feature 5.3.3: chain boundary syntax

Implement explicit chain boundary parsing and representation.

Acceptance tests:

- boundary syntax is accepted in CLI args and effects files.
- empty chains are rejected or documented.
- boundary rendering is deterministic in manifests.
- unsupported `newfile` and `restart` semantics report stable diagnostics until
  their leaf features are implemented.

---

### Milestone 5.4: input combiners

Implement multi-input pipeline combiners before more codecs:

#### Feature 5.4.1: concatenate combiner

Implement the concatenate combiner.

Acceptance tests:

- mono and stereo inputs.
- mismatched lengths.
- mismatched channel count behavior documented.
- SoX-ng golden comparison.
- full-chain test combining inputs before effects.

---

#### Feature 5.4.2: sequence combiner

Implement the sequence combiner.

Acceptance tests:

- mono and stereo inputs.
- sequence boundary behavior matches documented semantics.
- SoX-ng golden comparison.
- full-chain test combining inputs before effects.

---

#### Feature 5.4.3: mix combiner

Implement the mix combiner.

Acceptance tests:

- equal-length and mismatched-length inputs.
- clipping and normalization behavior documented.
- scalar-vs-SIMD tests for mixing kernels.
- SoX-ng golden comparison.

---

#### Feature 5.4.4: mix-power combiner

Implement the mix-power combiner.

Acceptance tests:

- equal-length and mismatched-length inputs.
- power scaling behavior documented.
- scalar-vs-SIMD tests for mixing kernels.
- SoX-ng golden comparison.

---

#### Feature 5.4.5: merge combiner

Implement the merge combiner.

Acceptance tests:

- mono-to-stereo merge.
- multichannel merge.
- mismatched length behavior documented.
- SoX-ng golden comparison.

---

#### Feature 5.4.6: multiply combiner

Implement the multiply combiner.

Acceptance tests:

- mono and stereo inputs.
- zero and one identity cases.
- scalar-vs-SIMD tests for multiply kernels.
- SoX-ng golden comparison.

---

### Milestone 5.5: automatic pipeline effects

Define and implement explicit equivalents for SoX-ng automatic behavior.

#### Feature 5.5.1: automatic channel conversion policy

Acceptance tests:

- no hidden behavior in library APIs.
- CLI defaults are documented.
- disabling automatic channel conversion is possible in tests.
- SoX-ng comparison tests record when SoX-ng auto-inserted channel conversion.

---

#### Feature 5.5.2: automatic sample-rate conversion policy

Acceptance tests:

- no hidden behavior in library APIs.
- CLI defaults are documented.
- disabling automatic rate conversion is possible in tests.
- SoX-ng comparison tests record when SoX-ng auto-inserted `rate`.

---

#### Feature 5.5.3: guard and norm pipeline behavior

Acceptance tests:

- guard behavior is explicit in library APIs.
- CLI `--guard` and `--norm` behavior is documented.
- SoX-ng golden tests cover representative clipping cases.

---

#### Feature 5.5.4: automatic dither insertion policy

Implement only after the `dither` effect exists.

Acceptance tests:

- dither insertion rules are explicit and testable.
- disabling dither is possible in tests.
- SoX-ng comparison tests record when SoX-ng auto-inserted `dither`.

---

## Effect coverage milestones

Implement effects in the following order. Each effect must satisfy the effect
test contract, include scalar and SIMD work where applicable, and update the
SoX-ng coverage entry.

### Milestone 6.1: complete existing SoX-ng semantics

#### Feature 6.1.1: `gain` headroom and reclaim options

Implement `gain -h` and `gain -r`.

Acceptance tests:

- manifest case per option.
- option interactions tested where SoX-ng documents combinations.
- scalar-vs-SIMD tests for gain kernels.
- SoX-ng golden comparisons.

---

#### Feature 6.1.2: `gain` normalize and limiter options

Implement `gain -n` and `gain -l`.

Acceptance tests:

- manifest case per option.
- limiter behavior documented and tested.
- scalar-vs-SIMD tests for gain kernels.
- SoX-ng golden comparisons.

---

#### Feature 6.1.3: `gain` channel equalize and balance options

Implement `gain -e`, `gain -b`, and `gain -B`.

Acceptance tests:

- stereo and multichannel fixtures.
- peak and RMS balance behavior documented.
- SoX-ng golden comparisons.

---

#### Feature 6.1.4: `fade` curve types

Implement `fade` curve types `q`, `h`, `t`, `l`, and `p`.

Acceptance tests:

- analytical envelope tests for each curve.
- scalar-vs-SIMD tests for envelope multiplication.
- SoX-ng golden comparisons.

---

#### Feature 6.1.5: `fade` stop position and fade-out length

Acceptance tests:

- fade-in only, fade-out only, and combined fade.
- exact output length behavior documented.
- chunk invariance.
- SoX-ng golden comparisons.

---

#### Feature 6.1.6: `dcshift` limiter gain

Acceptance tests:

- limiter threshold behavior documented.
- scalar-vs-SIMD tests for offset and limiter kernels.
- SoX-ng golden comparisons.

---

#### Feature 6.1.7: `pad` positioned padding

Implement multiple `length[@position]` entries.

Acceptance tests:

- exact editing behavior.
- overlapping or unsorted positions documented.
- SoX-ng golden comparisons.

---

#### Feature 6.1.8: `trim` multiple and relative positions

Acceptance tests:

- multiple positions.
- relative positions where compatible.
- exact editing behavior.
- SoX-ng golden comparisons.

---

### Milestone 6.2: volume, level, and simple modulation effects

#### Feature 6.2.1: `vol`

Acceptance tests:

- amplitude, power, and dB modes.
- limiter gain behavior.
- scalar-vs-SIMD tests for sample-wise kernels.
- SoX-ng golden comparisons.

---

#### Feature 6.2.2: `norm`

Acceptance tests:

- default and explicit level.
- silence behavior documented.
- SoX-ng golden comparisons.

---

#### Feature 6.2.3: `contrast`

Acceptance tests:

- default and non-default amount.
- analytical sanity tests.
- scalar-vs-SIMD tests for sample-wise kernels.
- SoX-ng golden comparisons.

---

#### Feature 6.2.4: `softvol`

Acceptance tests:

- volume, double-time, and headroom options.
- chunk invariance for ramps.
- scalar-vs-SIMD tests for gain application.
- SoX-ng golden comparisons.

---

#### Feature 6.2.5: `tremolo`

Acceptance tests:

- speed and depth options.
- phase continuity across chunks.
- scalar-vs-SIMD tests for modulation kernels.
- SoX-ng golden comparisons.

---

#### Feature 6.2.6: `overdrive`

Acceptance tests:

- gain and color options.
- clipping behavior documented.
- scalar-vs-SIMD tests for waveshaping kernels.
- SoX-ng golden comparisons.

---

#### Feature 6.2.7: `saturation`

Acceptance tests:

- `tanh`, `sqrt`, and `diode` modes.
- blend, offset, and mode-specific parameters.
- scalar-vs-SIMD tests for waveshaping kernels.
- SoX-ng golden comparisons.

---

#### Feature 6.2.8: `repeat`

Acceptance tests:

- finite repeat counts.
- reject or explicitly block infinite repeat in library-safe contexts.
- exact frame count tests.
- SoX-ng golden comparisons.

---

### Milestone 6.3: channel and mixing effects

#### Feature 6.3.1: `channels`

Acceptance tests:

- mono-to-stereo, stereo-to-mono, and multichannel fixtures.
- channel identity fixtures prove routing and gain.
- SoX-ng golden comparisons.

---

#### Feature 6.3.2: `remix` basic routing

Acceptance tests:

- selecting, dropping, and duplicating input channels.
- silent channel `0`.
- channel identity fixtures.
- SoX-ng golden comparisons.

---

#### Feature 6.3.3: `remix` gain modifiers

Acceptance tests:

- `v`, `p`, and `i` modifiers.
- `-a`, `-m`, and `-p` options.
- scalar-vs-SIMD tests for mixing kernels.
- SoX-ng golden comparisons.

---

#### Feature 6.3.4: `swap`

Acceptance tests:

- stereo pair swap.
- odd channel count behavior documented.
- SoX-ng golden comparisons.

---

#### Feature 6.3.5: `oops`

Acceptance tests:

- stereo phase cancellation fixture.
- mono and non-stereo behavior documented.
- SoX-ng golden comparisons.

---

#### Feature 6.3.6: `centercut` core

Acceptance tests:

- left, right, and center separation fixtures.
- window-size behavior documented.
- SoX-ng golden comparisons.

---

#### Feature 6.3.7: `centercut` options

Implement `-a`, `-b`, and `-w`.

Acceptance tests:

- option-specific golden cases.
- invalid window size diagnostics.
- scalar-vs-SIMD tests for mix kernels where applicable.

---

### Milestone 6.4: biquad and tone filters

#### Feature 6.4.1: biquad primitive

Acceptance tests:

- coefficient construction tests.
- impulse and step responses.
- chunk invariance with filter state.
- scalar-vs-SIMD tests where the chosen structure permits it.

---

#### Feature 6.4.2: `biquad` effect

Acceptance tests:

- externally supplied coefficients.
- invalid coefficient diagnostics.
- SoX-ng golden comparisons.

---

#### Feature 6.4.3: RBJ coefficient helpers

Acceptance tests:

- low-pass, high-pass, shelving, peaking, all-pass, band-pass, and band-reject
  coefficient fixtures.
- finite coefficients for valid inputs.
- invalid frequency and width diagnostics.

---

#### Feature 6.4.4: `allpass`

Acceptance tests:

- `-1` and `-2` modes.
- impulse and frequency response.
- SoX-ng golden comparisons.

---

#### Feature 6.4.5: `band`

Acceptance tests:

- default and `-n` mode.
- impulse and frequency response.
- SoX-ng golden comparisons.

---

#### Feature 6.4.6: `bandpass`

Acceptance tests:

- default and `-c` mode.
- frequency response.
- SoX-ng golden comparisons.

---

#### Feature 6.4.7: `bandreject`

Acceptance tests:

- frequency response.
- chunk invariance.
- SoX-ng golden comparisons.

---

#### Feature 6.4.8: `bass`

Acceptance tests:

- gain, frequency, and width options.
- frequency response.
- SoX-ng golden comparisons.

---

#### Feature 6.4.9: `treble`

Acceptance tests:

- gain, frequency, and width options.
- frequency response.
- SoX-ng golden comparisons.

---

#### Feature 6.4.10: `equalizer`

Acceptance tests:

- frequency, width, and gain options.
- frequency response.
- SoX-ng golden comparisons.

---

#### Feature 6.4.11: `lowpass`

Acceptance tests:

- `-1` and `-2` modes.
- DC pass and high-frequency rejection.
- SoX-ng golden comparisons.

---

#### Feature 6.4.12: `highpass`

Acceptance tests:

- `-1` and `-2` modes.
- DC rejection.
- SoX-ng golden comparisons.

---

#### Feature 6.4.13: `deemph`

Acceptance tests:

- documented CD de-emphasis response.
- SoX-ng golden comparisons.

---

#### Feature 6.4.14: `riaa`

Acceptance tests:

- documented RIAA response.
- SoX-ng golden comparisons.

---

### Milestone 6.5: delay, echo, and modulation effects

#### Feature 6.5.1: `delay`

Acceptance tests:

- per-channel delay positions.
- impulse response verifies delay placement.
- short-input tests shorter than delay lines.
- tail flush behavior documented.
- SoX-ng golden comparisons.

---

#### Feature 6.5.2: `echo`

Acceptance tests:

- gain-in, gain-out, delay, and decay.
- impulse response verifies feedback placement.
- chunk invariance including tail output.
- SoX-ng golden comparisons.

---

#### Feature 6.5.3: `echos`

Acceptance tests:

- multiple delay and decay pairs.
- tail flush behavior.
- SoX-ng golden comparisons.

---

#### Feature 6.5.4: `chorus` core

Acceptance tests:

- default options.
- sine and triangle modulation.
- chunk invariance.
- SoX-ng golden comparisons.

---

#### Feature 6.5.5: `chorus` interpolation and multi-delay options

Acceptance tests:

- none, linear, and quadratic interpolation.
- multiple delay voices.
- scalar-vs-SIMD tests for wet/dry mix and interpolation kernels.
- SoX-ng golden comparisons.

---

#### Feature 6.5.6: `flanger`

Acceptance tests:

- delay, depth, regen, width, speed, shape, phase, and interpolation options.
- chunk invariance including modulation phase.
- SoX-ng golden comparisons.

---

#### Feature 6.5.7: `phaser`

Acceptance tests:

- interpolation, wave shape, gain, delay, regen, and speed options.
- chunk invariance including modulation phase.
- SoX-ng golden comparisons.

---

#### Feature 6.5.8: `reverb`

Acceptance tests:

- default and representative parameter sets.
- wet-only mode.
- tail length documented.
- SoX-ng golden comparisons.

---

### Milestone 6.6: sample-rate and time-domain effects

#### Feature 6.6.1: `downsample`

Acceptance tests:

- output length formula.
- factor validation.
- SoX-ng golden comparisons.

---

#### Feature 6.6.2: `upsample`

Acceptance tests:

- output length formula.
- zero-stuffing behavior.
- SoX-ng golden comparisons.

---

#### Feature 6.6.3: `speed`

Acceptance tests:

- factor and cents syntax where compatible.
- output rate and length behavior documented.
- SoX-ng golden comparisons.

---

#### Feature 6.6.4: `rate` specification and scaffolding

Acceptance tests:

- spec documents quality levels, phase behavior, latency, output length, and
  alias rejection targets.
- skipped tests name the missing implementation reason.

---

#### Feature 6.6.5: `rate` quick and low-quality modes

Acceptance tests:

- output length formulas.
- alias rejection and pass-band checks for implemented modes.
- scalar-vs-SIMD tests for resampler inner loops.
- SoX-ng golden comparisons.

---

#### Feature 6.6.6: `rate` high-quality modes

Acceptance tests:

- `-m`, `-h`, `-v`, and related quality modes implemented incrementally.
- phase behavior documented.
- SoX-ng golden comparisons.

---

#### Feature 6.6.7: `rate` override options

Acceptance tests:

- one option family per test case.
- unsupported options fail with stable diagnostics until implemented.
- SoX-ng golden comparisons.

---

#### Feature 6.6.8: `stretch`

Acceptance tests:

- factor, window, fade, shift, and fading options.
- output length formula.
- SoX-ng golden comparisons.

---

#### Feature 6.6.9: `tempo` core

Acceptance tests:

- default, music, speech, and linear modes.
- output length formula.
- SoX-ng golden comparisons.

---

#### Feature 6.6.10: `tempo` tuning options

Acceptance tests:

- segment, search, overlap, and quick-search options.
- boundary diagnostics.
- SoX-ng golden comparisons.

---

#### Feature 6.6.11: `pitch`

Acceptance tests:

- pitch shift in cents.
- segment, search, overlap, and quick-search options.
- SoX-ng golden comparisons.

---

#### Feature 6.6.12: `bend`

Acceptance tests:

- single and multiple bend segments.
- frame-rate and oversample options.
- SoX-ng golden comparisons.

---

#### Feature 6.6.13: `splice`

Acceptance tests:

- half-sine, triangular, and quarter-sine modes.
- position, excess, and leeway options.
- SoX-ng golden comparisons.

---

### Milestone 6.7: dynamics, silence, and noise effects

#### Feature 6.7.1: `compand` parser and transfer function

Acceptance tests:

- attack/decay parsing.
- soft-knee and transfer point parsing.
- transfer curve tests.
- invalid syntax diagnostics.

---

#### Feature 6.7.2: `compand` processor

Acceptance tests:

- envelope follower tests with known attack and decay curves.
- delay and gain behavior.
- chunk invariance with lookahead.
- SoX-ng golden comparisons.

---

#### Feature 6.7.3: `mcompand`

Acceptance tests:

- crossover parsing.
- per-band compand behavior.
- SoX-ng golden comparisons.

---

#### Feature 6.7.4: `loudness`

Acceptance tests:

- gain, reference, and filter-size options.
- SoX-ng golden comparisons.

---

#### Feature 6.7.5: `silence`

Acceptance tests:

- above and below period rules.
- threshold boundary tests.
- SoX-ng golden comparisons.

---

#### Feature 6.7.6: `vad` core

Acceptance tests:

- trigger level and timing options.
- speech-like and silence fixtures.
- SoX-ng golden comparisons.

---

#### Feature 6.7.7: `vad` advanced options

Acceptance tests:

- noise estimate, measurement, and filter options.
- stable diagnostics for invalid ranges.
- SoX-ng golden comparisons.

---

#### Feature 6.7.8: `noiseprof`

Acceptance tests:

- generated profile is deterministic.
- profile output format documented.
- SoX-ng golden comparisons where output format aligns.

---

#### Feature 6.7.9: `noisered`

Acceptance tests:

- generated profile round trips from `noiseprof`.
- amount option.
- speech-like and noise fixtures.
- SoX-ng golden comparisons.

---

### Milestone 6.8: FIR, analysis, generation, and dither effects

#### Feature 6.8.1: `fir` coefficient input

Acceptance tests:

- coefficient file and inline coefficients.
- invalid coefficient diagnostics.
- impulse response tests.

---

#### Feature 6.8.2: `fir` streaming processor

Acceptance tests:

- convolution tests.
- chunk invariance.
- scalar-vs-SIMD tests for convolution kernels.
- SoX-ng golden comparisons.

---

#### Feature 6.8.3: `firfit`

Acceptance tests:

- knots file and inline frequency/gain pairs.
- generated FIR response sanity checks.
- SoX-ng golden comparisons.

---

#### Feature 6.8.4: `hilbert`

Acceptance tests:

- tap count option.
- phase-shift spectral checks.
- SoX-ng golden comparisons.

---

#### Feature 6.8.5: `sinc` low-pass and high-pass

Acceptance tests:

- attenuation, beta, phase, transition bandwidth, and tap options.
- spectral checks.
- SoX-ng golden comparisons.

---

#### Feature 6.8.6: `sinc` band-pass and band-reject

Acceptance tests:

- two-frequency forms.
- spectral checks.
- SoX-ng golden comparisons.

---

#### Feature 6.8.7: `dither` TPDF and sloped TPDF

Acceptance tests:

- deterministic random tests with fixed seeds.
- precision option.
- SoX-ng golden comparisons with `-R` and dither enabled.

---

#### Feature 6.8.8: `dither` noise shaping

Acceptance tests:

- supported shaping filters.
- spectrum checks.
- SoX-ng golden comparisons.

---

#### Feature 6.8.9: `stat`

Acceptance tests:

- text output options.
- JSON output option.
- SoX-ng output comparison where stable.

---

#### Feature 6.8.10: `stats`

Acceptance tests:

- window, bits, hex, scale, and JSON options.
- SoX-ng output comparison where stable.

---

#### Feature 6.8.11: `synth` basic waveforms

Acceptance tests:

- sine, square, triangle, sawtooth, and trapezium.
- length, offset, phase, and normalization.
- SoX-ng golden comparisons.

---

#### Feature 6.8.12: `synth` noise, sweep, and combine modes

Acceptance tests:

- white, TPDF, pink, and brown noise.
- linear, quadratic, exponential, and stepped sweeps.
- create, mix, amod, fmod, and vdelay combine modes.
- SoX-ng golden comparisons.

---

### Milestone 6.9: specialized and integration effects

Implement or explicitly block with a documented reason:

#### Feature 6.9.1: `dolbyb` feasibility and spec

Acceptance tests:

- coverage entry explains implementation strategy or blocker.
- CLI diagnostic is stable if blocked.

---

#### Feature 6.9.2: `dolbyb` implementation

Implement only if Feature 6.9.1 records a safe implementation path.

Acceptance tests:

- encode and decode modes.
- filter and threshold options.
- SoX-ng golden comparisons.

---

#### Feature 6.9.3: `dop`

Acceptance tests:

- DSD-over-PCM packing behavior.
- SoX-ng golden comparisons.

---

#### Feature 6.9.4: `earwax`

Acceptance tests:

- stereo-only behavior documented.
- SoX-ng golden comparisons.

---

#### Feature 6.9.5: `ladspa` host or stable block

Acceptance tests:

- if implemented, plugin loading errors are typed and safe.
- if blocked, CLI diagnostics are stable and actionable.
- coverage entry explains the dependency and safety boundary.

---

#### Feature 6.9.6: `sdm` feasibility and spec

Acceptance tests:

- coverage entry explains implementation strategy or blocker.
- CLI diagnostic is stable if blocked.

---

#### Feature 6.9.7: `sdm` implementation

Implement only if Feature 6.9.6 records a safe implementation path.

Acceptance tests:

- filter, order, path count, and latency options.
- SoX-ng golden comparisons.

---

## Format support milestone

Additional formats are intentionally after effect and pipeline coverage. Add a
format earlier only when it is required to test an effect that cannot be tested
faithfully through WAV PCM16.

Each format leaf feature uses the format acceptance tests below.

### Milestone 7.1: richer WAV support

#### Feature 7.1.1: WAV PCM8

#### Feature 7.1.2: WAV PCM24

#### Feature 7.1.3: WAV PCM32

#### Feature 7.1.4: WAV float32

#### Feature 7.1.5: WAV float64

#### Feature 7.1.6: WAV u-law and A-law

#### Feature 7.1.7: WAV RIFX

### Milestone 7.2: raw formats

#### Feature 7.2.1: raw signed and unsigned PCM

#### Feature 7.2.2: raw float32 and float64

#### Feature 7.2.3: raw endian, bit-order, and nibble-order options

### Milestone 7.3: AIFF formats

#### Feature 7.3.1: AIFF PCM

#### Feature 7.3.2: AIFC encodings

### Milestone 7.4: FLAC

#### Feature 7.4.1: FLAC decode

#### Feature 7.4.2: FLAC encode

### Milestone 7.5: AU/SND

#### Feature 7.5.1: AU/SND

### Milestone 7.6: external decode strategy

#### Feature 7.6.1: external-tool-backed decode strategy

Format acceptance tests for each format leaf feature:

- decode and encode fixtures where the format supports both.
- unsupported encoding diagnostics.
- metadata preservation where the format has metadata.
- SoX-ng decode comparison into a common WAV or raw-float representation.
- effect pipeline tests using the new format only after standalone codec tests pass.

---

## Python and package milestone

Do not add PyO3 bindings until effect pipeline behavior is stable.

Future work:

- Python package via PyO3.
- NumPy-compatible buffer interface.
- stable public crate release.

Before then, Python remains a test harness for corpus generation, golden
comparison, metrics, and failure artifacts.

---

## Acceptance checklist template

Every feature should add or update a checklist like this in the relevant issue, PR, or commit note:

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

Tests:
[ ] Unit tests
[ ] Doc tests
[ ] Integration tests
[ ] CLI and typed API equivalence tests, if applicable
[ ] SoX-ng golden tests, if applicable
[ ] Analytical tests, if applicable
[ ] Property/metamorphic tests, if applicable
[ ] Chunk invariance tests, if applicable
[ ] Scalar-vs-SIMD tests, or SIMD N/A reason checked
[ ] Python uv pytest tests, if applicable

Quality gate:
[ ] cargo fmt --all --check
[ ] cargo clippy --workspace --all-targets --all-features -- -D warnings
[ ] cargo test --workspace --all-features
[ ] cargo test --doc --workspace
[ ] uv run pytest, if Python tests exist
[ ] SoX-ng golden test command, if golden manifests exist
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

SIMD is part of the normal implementation path after Feature 3.3, not a late
optimization phase. For every new sample-processing feature:

1. define the scalar reference behavior first,
2. add or extend the backend trait for the effect's core kernel,
3. implement the SIMD backend in the same feature when the kernel is
   data-parallel,
4. add forced scalar and forced SIMD tests,
5. add scalar-vs-SIMD differential tests,
6. document a SIMD N/A reason only when the algorithm has no useful vectorizable
   kernel.

Benchmarks are still required for performance-sensitive kernels, but lack of a
benchmark is not a reason to skip the SIMD backend for an otherwise vectorizable
effect.

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
