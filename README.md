# Auralis

> A deterministic, testable, batch-oriented audio DSP engine for Rust and the command line.

Auralis is a from-scratch Rust audio processing project. Its goal is not to be a literal clone of SoX, but to preserve the useful shape of classic command-line audio tools while rebuilding the core around deterministic behavior, typed APIs, exhaustive tests, and long-term maintainability.

The intended standard is:

- **SQLite-like reliability**: small surface area, explicit invariants, deterministic behavior, extensive regression tests, and conservative evolution.
- **FFmpeg-like command-line composability**: usable from scripts, pipelines, CI, and batch jobs.
- **Rust-first library design**: ergonomic typed APIs, chainable builders, clear errors, and documented numerical behavior.
- **NumPy-like documentation discipline**: every public API should describe parameters, return values, examples, errors, determinism, numerical tolerance, and edge cases.
- **Future Python package compatibility**: the internal model should remain friendly to a future `pyo3` / `maturin` binding, but Python support is not part of the initial milestone.

Initial scope is deliberately narrow: **WAV only**. Other audio formats are represented by codec placeholders and will be added after the WAV pipeline and DSP test system are stable.

---

## Project status

Auralis is currently pre-alpha. The repository starts empty except for `LICENSE`; this README defines the intended architecture and development rules.

The nearby `sox_ng` checkout is used only as a reference implementation for golden tests. It is not vendored into Auralis and should not shape the internal architecture.

Expected local layout during development:

```text
~/code/sox-rs/
├── auralis/     # this repository
└── sox_ng/      # reference implementation, already globally installed
```

The test harness assumes `sox_ng` is available in `PATH`, or explicitly configured through:

```bash
export AURALIS_SOX_NG_BIN=sox_ng
```

---

## Design principles

### 1. Determinism by default

Auralis should produce reproducible output for the same input, parameters, version, target backend, and documented floating-point tolerance.

Rules:

- No implicit random seed in DSP paths.
- No timestamp-dependent output unless explicitly requested.
- Dither and stochastic algorithms must expose explicit seed configuration.
- Metadata that is not semantically required should not affect DSP regression tests.
- Tests should compare decoded sample data, not container bytes, unless the test is specifically about container serialization.

### 2. Library first, CLI second

The CLI is important, but it should be a thin layer over the Rust library.

The library API should support:

```rust
use auralis::{AudioFile, Pipeline};

fn main() -> auralis::Result<()> {
    AudioFile::open_wav("input.wav")?
        .into_pipeline()
        .gain_db(-3.0)
        .trim_seconds(0.0..10.0)
        .fade_out_seconds(0.25)
        .write_wav("output.wav")?;

    Ok(())
}
```

The command-line equivalent should be composable:

```bash
auralis -i input.wav -o output.wav gain -3 trim 0 10 fade-out 0.25
```

A structured pipeline form should also exist for reproducible batch workflows:

```bash
auralis run pipeline.toml
```

Example `pipeline.toml`:

```toml
input = "input.wav"
output = "output.wav"

[[effects]]
type = "gain"
db = -3.0

[[effects]]
type = "trim"
start_seconds = 0.0
end_seconds = 10.0

[[effects]]
type = "fade_out"
duration_seconds = 0.25
```

### 3. Typed effect configuration

Command-line strings should be parsed into typed Rust structures as early as possible.

Do not keep long-lived effect state as untyped strings.

Example:

```rust
pub struct Gain {
    pub db: f32,
}

pub struct Trim {
    pub start: TimePosition,
    pub end: Option<TimePosition>,
}

pub struct Lowpass {
    pub cutoff_hz: Hertz,
    pub design: FilterDesign,
}
```

### 4. DSP and file I/O are separate concerns

WAV decoding, sample conversion, buffer layout, effect execution, and WAV encoding are separate layers.

Do not let file format details leak into DSP kernels.

The DSP core should operate on an internal format:

```text
planar f32 audio blocks
```

Suggested layout:

```rust
pub struct AudioBuffer {
    pub spec: AudioSpec,
    pub frames: usize,
    pub data: Vec<f32>, // planar: channel 0 frames, channel 1 frames, ...
}
```

Rationale:

- `f32` is the primary format for modern DSP.
- Planar layout is friendlier for SIMD, filters, channel-wise processing, and future Python/NumPy views.
- Interleaved layout is a codec/transport concern, not the internal default.

### 5. Public APIs must be boring and explicit

Auralis should avoid clever interfaces. Interfaces should be predictable, inspectable, and hard to misuse.

Each public function should document:

- Inputs and units.
- Output shape and sample format.
- Error conditions.
- Whether it is deterministic.
- Numerical tolerance or exactness expectations.
- Whether processing is streaming-safe.
- Complexity or memory behavior when relevant.
- Examples.

---

## Initial architecture

The repository should start as a Rust workspace:

```text
auralis/
├── Cargo.toml
├── README.md
├── DEVELOPMENT.md
├── crates/
│   ├── auralis-core/
│   ├── auralis-codec/
│   ├── auralis-wav/
│   ├── auralis-dsp/
│   ├── auralis-effects/
│   ├── auralis-simd/
│   ├── auralis-cli/
│   └── auralis-testkit/
├── tools/
│   └── pytest/
├── tests/
│   ├── golden/
│   ├── corpus/
│   └── integration/
└── benches/
```

### `auralis-core`

Defines the stable vocabulary of the project:

- `AudioSpec`
- `AudioBuffer`
- `AudioBlockView`
- `SampleRate`
- `ChannelCount`
- `SampleFormat`
- `TimePosition`
- `Hertz`
- `Decibels`
- `AuralisError`
- `Result<T>`

This crate should have no dependency on CLI, WAV libraries, Python, or SIMD backends.

### `auralis-codec`

Defines codec traits and format-neutral interfaces:

- `AudioReader`
- `AudioWriter`
- `CodecRegistry`
- `DecodedAudio`
- `EncodedAudioFormat`

It should include placeholders for future formats:

```rust
pub enum CodecKind {
    Wav,
    Flac,  // placeholder
    Aiff,  // placeholder
    Ogg,   // placeholder
    Mp3,   // placeholder
    Raw,   // placeholder
}
```

Only WAV is implemented initially.

### `auralis-wav`

Implements WAV reading and writing.

Initial WAV scope:

- PCM 16-bit little-endian.
- PCM 24-bit little-endian if practical in the first pass; otherwise reserve the API and explicitly return `UnsupportedFormat`.
- IEEE float 32-bit if practical; otherwise reserve the API and explicitly return `UnsupportedFormat`.
- Mono and stereo first; multi-channel support should be designed but may be gated behind tests.

The WAV module should expose decoded planar `f32` buffers to the rest of the system.

Candidate crate: `hound`, wrapped behind Auralis traits rather than exposed directly.

### `auralis-dsp`

Contains scalar DSP primitives and reference implementations:

- gain
- dc shift
- clipping
- fades
- channel mixing
- FIR primitives
- biquad primitives
- resampler primitives, added later
- metrics used by tests where appropriate

Scalar implementations are the source of truth for SIMD differential tests.

### `auralis-effects`

Contains typed effect processors built from DSP primitives:

- `Gain`
- `Trim`
- `Pad`
- `Reverse`
- `Remix`
- `Fade`
- later: `Lowpass`, `Highpass`, `Biquad`, `Rate`, `Compand`, `Delay`, `Reverb`, `Silence`

Effect implementations should be block-based and streaming-aware from the beginning, even if the initial CLI processes whole files.

### `auralis-simd`

Contains optional SIMD acceleration.

Primary selected SIMD abstraction:

```toml
rten-simd = "0.24"
```

Reasons:

- stable Rust
- portable SIMD abstraction
- runtime ISA dispatch
- supports AVX2, AVX-512, Arm Neon, and WebAssembly SIMD
- suitable for custom 1D slice kernels

`rten-simd` must not leak into the public API. Auralis should define its own kernel traits and keep SIMD as an implementation detail.

Initial SIMD targets:

- `gain_f32`
- `mix2_f32`
- `clip_f32`
- `i16_to_f32`
- `f32_to_i16`
- later: FIR and polyphase resampling inner loops

Do not prioritize SIMD for state-machine-heavy or recursive algorithms at first.

### `auralis-cli`

Provides the `auralis` binary.

Initial CLI goals:

```bash
auralis --version
auralis inspect input.wav
auralis -i input.wav -o output.wav copy
auralis -i input.wav -o output.wav gain -3
auralis -i input.wav -o output.wav trim 0 10
auralis run pipeline.toml
```

Candidate crate: `clap` with derive support.

### `auralis-testkit`

Contains test utilities shared by Rust tests and Python tests:

- synthetic corpus generation
- raw f32 helpers
- WAV decode helpers
- metric calculation
- golden test manifest handling
- SoX-ng command wrapper
- tolerance definitions
- failure artifact generation

### `auralis-python` future placeholder

Python support should be planned but not implemented until the Rust API is stable.

Future direction:

- `pyo3` for bindings
- `maturin` for package builds
- NumPy-compatible buffer exposure
- Python package name: `auralis`

Do not introduce Python bindings during the initial WAV and core DSP milestones.

---

## Foundation library choices

| Area | Choice | Status | Reason |
|---|---|---:|---|
| Language | Rust | primary | Safety, typed APIs, testing, batch orchestration |
| CLI | `clap` | initial | Mature Rust CLI parser |
| Errors | `thiserror` for library, `anyhow` for CLI/tests | initial | Explicit library errors, ergonomic binary errors |
| Serialization | `serde`, `toml` | initial | Structured pipeline manifests |
| WAV | `hound` behind wrapper | initial | Simple WAV reader/writer; implementation detail only |
| SIMD | `rten-simd` | selected | Stable Rust, portable SIMD, runtime dispatch |
| Testing | Rust tests + Python pytest | selected | Rust for unit tests, Python for numerical/oracle tests |
| Python environment | `uv` | selected | Python tool/project management and reproducible test env |
| Property tests | `proptest` | selected | Randomized invariant checks |
| Benchmarks | `criterion` | selected | Statistical Rust microbenchmarks |
| Python bindings | `pyo3` + `maturin` | future | Native Python package after Rust API stabilizes |

---

## Testing philosophy

Auralis should be developed as a test-locked system.

A feature is not complete when it compiles. It is complete only when it passes its full acceptance test set against:

1. its scalar reference behavior,
2. analytical expectations where applicable,
3. SoX-ng golden behavior where applicable,
4. chunked streaming behavior,
5. edge cases,
6. documentation examples.

No new feature should be started until the current feature has passed its acceptance tests.

---

## Layered test design

### L0: deterministic corpus

Generate small, controlled test signals:

- silence
- impulse
- step
- sine
- sweep
- seeded noise
- full-scale signal
- near-zero signal
- odd-length buffers
- mono and stereo buffers

The corpus should be generated programmatically. Do not rely primarily on hand-picked music files.

### L1: WAV I/O correctness

Tests should validate:

- WAV header parsing
- supported sample formats
- sample-rate preservation
- channel-count preservation
- sample conversion into internal planar `f32`
- write-read round trip
- rejection of unsupported formats with clear errors

For WAV container tests, byte-level comparison is allowed only when metadata is fully controlled. Otherwise compare decoded samples and metadata fields.

### L2: golden regression against `sox_ng`

For implemented effects, compare Auralis output with `sox_ng` output using controlled inputs.

Use repeatable SoX-ng invocation for reference generation:

```bash
sox_ng -R -D input.wav output.wav gain -3
```

For DSP comparison, prefer decoded samples or raw float output over container bytes.

Golden tests should record:

- command
- input corpus ID
- SoX-ng version
- Auralis version or commit
- metric thresholds
- failure artifacts

### L3: analytical DSP tests

Where the effect has a mathematical model, test that model directly.

Examples:

- `gain`: multiply by `10^(db / 20)`
- `dcshift`: add constant offset
- `trim`: exact frame interval
- `reverse`: exact frame order, including stereo frame grouping
- `fade`: expected envelope shape
- filters later: frequency response, passband ripple, stopband attenuation
- resampler later: output length, alias rejection, passband behavior

Golden tests prove compatibility. Analytical tests prove correctness.

Both are required when applicable.

### L4: property and metamorphic tests

Every effect should have basic properties:

- finite input produces finite output unless explicitly documented otherwise
- silence does not produce unexpected energy for effects that should preserve silence
- identity parameters behave as identity
- output length is correct
- no panics on edge lengths

Examples:

- `gain 0 dB` is identity
- `reverse` twice returns the original signal
- `trim` over the full range is identity
- `pad 0` is identity
- `gain +6 dB` followed by `gain -6 dB` approximately returns the original signal within tolerance

### L5: chunk invariance

Every streaming-capable effect should behave the same whether data is processed whole or in chunks.

Required chunk sizes:

```text
1, 2, 7, 15, 16, 17, 31, 32, 33, 64, 255, 1024, random seeded chunks
```

This layer is mandatory for stateful effects and useful even for simple effects.

### L6: scalar vs SIMD differential tests

Every SIMD kernel must compare against scalar reference output.

Required cases:

- empty slice
- one sample
- vector width minus one
- vector width
- vector width plus one
- odd lengths
- unaligned buffers where applicable
- mono and stereo layouts
- near-zero values
- near-clipping values
- seeded random values

SIMD is not accepted on benchmark results alone. It must pass differential correctness first.

### L7: fuzzing, sanitizers, and coverage

Rust reduces many memory risks, but fuzzing is still required for:

- WAV parser behavior
- pipeline parser behavior
- effect argument parsing
- edge-case chunking
- unsupported format rejection

Coverage gates should focus on touched code and DSP modules, not a misleading whole-repository percentage.

---

## Numerical metrics

Common comparison metrics:

```text
max_abs_error
rms_error
snr_db
length_diff
peak_error
dc_offset_error
spectral_error_db
```

Simple deterministic effects should have strict thresholds. Stateful or numerical effects may use wider, documented tolerances.

Tolerance is part of the API contract. Do not hide it in test code only.

---

## Documentation standard

Auralis documentation should be unusually explicit.

Every public effect should document:

```text
What it does
Parameters and units
Valid parameter range
Invalid parameter behavior
Streaming behavior
Latency
Output length behavior
Determinism
Numerical tolerance
SoX-ng compatibility status
Examples in Rust
Examples in CLI
Test coverage summary
```

Example documentation shape:

```rust
/// Applies a constant gain to every sample.
///
/// # Parameters
///
/// - `db`: gain in decibels. `0.0` is identity.
///
/// # Determinism
///
/// Deterministic for the same backend and input. Scalar and SIMD backends are
/// required to match within the documented tolerance.
///
/// # Numerical behavior
///
/// The linear multiplier is `10^(db / 20)`. This effect does not normalize and
/// does not apply automatic limiting.
///
/// # Examples
///
/// ```rust
/// # fn main() -> auralis::Result<()> {
/// auralis::AudioFile::open_wav("in.wav")?
///     .into_pipeline()
///     .gain_db(-3.0)
///     .write_wav("out.wav")?;
/// # Ok(())
/// # }
/// ```
```

---

## Development commands

Rust:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --doc --workspace
cargo bench
```

Python test environment must use `uv`:

```bash
cd tools/pytest
uv sync
uv run pytest
```

Do not use ad-hoc system `pip` installs for project tests.

---

## First milestone

The first meaningful milestone is not “many effects”. It is a tested pipeline.

Milestone 1 acceptance:

- Rust workspace exists.
- WAV read/write works for PCM16 mono/stereo.
- Internal planar `f32` representation exists.
- CLI can inspect WAV files.
- CLI can copy WAV input to WAV output through the internal pipeline.
- CLI can apply `gain`.
- Library supports equivalent chainable calls.
- Golden tests compare `gain` against `sox_ng`.
- Analytical tests verify `gain` math.
- Chunk invariance passes for `gain`.
- Documentation examples compile.
- Python pytest harness runs under `uv`.

---

## Non-goals for the initial phase

- Full SoX feature parity.
- Real-time audio I/O.
- GPU acceleration.
- Full codec support.
- Python package release.
- Perfect byte-for-byte WAV reproduction across arbitrary metadata.
- SIMD before scalar correctness.

---

## License

See `LICENSE`.
