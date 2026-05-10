# Auralis Testing Notes

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

The checked-in coverage gate is
`doc/development/05-layered-coverage.toml`. It records the L0-L7 status, linked
tests, and N/A reason for each implemented effect and pipeline primitive. Run
`python3 tools/check_layered_coverage.py` after changing effect, combiner,
policy, parser, or test coverage so future feature work cannot drift away from
the matrix.

This matrix is a coverage contract, not a line-coverage report. It must not be
used as a substitute for `cargo llvm-cov` output when a feature changes
non-trivial Rust behavior.

## Rust and Python Responsibilities

Rust and `auralis-testkit` are the default home for durable behavior tests.
Prefer adding new corpus cases, golden manifest parsing, SoX-ng command
rendering, metrics, chunk invariance, parser coverage, and complex pipeline
scenarios to `auralis-testkit` and Rust integration tests when practical.

Python remains an auxiliary layer for cross-tool execution, numerical array
work, failure report generation, and compatibility checks that are still more
ergonomic with `numpy`, `scipy`, and `pytest`. Python tests must use `uv`, but
new feature behavior must not depend on Python as the only critical
verification path unless the feature explicitly records why a Rust/testkit test
would be impractical.

Duplicate Python golden tests should either migrate into the manifest-driven
Rust/testkit surface or be reduced to a small smoke test. The project should
avoid maintaining two independent golden definitions for the same basic effect.

## SoX-ng Oracle Requirements

Local exploratory pytest runs may skip SoX-ng golden cases when `sox_ng` is not
installed. Release validation, gnhf validation, and CI-style golden jobs must
treat a missing SoX-ng oracle as a failure. Set `AURALIS_SOX_NG_BIN` explicitly
for those jobs.

Standalone effect coverage is not enough for pipeline confidence. New
pipeline-sensitive work should add complex golden cases that combine multiple
axes, such as multi-input combine plus effects, effects-file execution plus
positional equivalence, explicit output policies with user effects, long-window
effects, and `:` boundary rendering.

Complex pipeline golden manifests are a later acceptance requirement for
pipeline-sensitive milestones; do not treat standalone effect golden coverage
as sufficient once behavior depends on ordering, boundaries, combine modes, or
output policies.
The current chain golden gate includes a multi-input `mix` pipeline followed by
user effects and validates both positional CLI arguments and the equivalent
effects-file path against SoX-ng. Output-policy and chain-boundary stress cases
remain future complex-pipeline coverage work.

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
- short files shorter than filter windows or delay lines

The corpus is generated programmatically in `auralis-testkit::corpus` and
`auralis_testkit.corpus` from stable IDs such as `l0/sine_mono_32`; golden
manifests can reference those IDs through `corpus_id` or `corpus_ids`. Do not
rely primarily on hand-picked music files.

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
- failure artifacts using `auralis.golden.failure.v1`

### L3: analytical DSP tests

Where the effect has a mathematical model, test that model directly.

Examples:

- `gain`: multiply by `10^(db / 20)`
- `norm`: scan the whole effect input, then scale non-silent audio to the target
  peak level
- `vol`: amplitude, power, or dB scaling with immediate effect-level clipping
  and optional SoX-ng limiter-gain shaping
- `contrast`: SoX-ng phase contrast enhancement with an optional amount
- `softvol`: per-frame soft volume control that lowers the current multiplier
  before clipping and can recover upward over a configured doubling time
- `loudness`: ISO 226 equal-loudness FIR compensation with SoX-ng's gain,
  reference-level, and half-length arguments
- `silence`: leading, trailing, and restart-based middle-silence trimming with
  sample-count or seconds durations and percent or dB thresholds
- `vad`: deterministic leading non-voice trimming with SoX-ng-style advanced
  option parsing, a frame-domain trigger, configurable pre-trigger retention,
  and quiet-gap tolerance
- `compand`: envelope-followed dB transfer with shared or per-channel attack
  and decay, optional post gain, initial volume, and look-ahead delay
- `mcompand`: multiband companding with quoted compand band groups, ascending
  crossovers, and scalar Linkwitz-Riley-style band splitting
- `noiseprof`: pass-through 2048-point FFT log-power profile collection with
  SoX-ng-style text rendering for each channel
- `noisered`: scalar overlap-window spectral noise reduction from
  SoX-ng-style profile text with configurable amount
- `stat`: pass-through sample statistics analysis with scale, RMS scaling,
  volume-only, and JSON report options
- `stats`: pass-through overall and per-channel sample statistics analysis
  with signed-bit, hexadecimal, floating scale, window-time, and JSON report
  options
- `synth`: waveform and noise generation with optional length, deterministic
  sine, square, sawtooth, triangle, trapezium, exp, white, TPDF, pink, and brown
  noise forms, plus sweep and input-combine modes
- `fir`: coefficient input parsing from stdin, one coefficient-file path, or
  inline finite coefficients, with `#` comments in coefficient text
- `firfit`: frequency/gain knot parsing from stdin, one knot-file path, or
  inline pairs, with scalar FIR coefficient design for fitted responses
- `sinc`: low-pass, high-pass, band-pass, and band-reject Kaiser-windowed FIR
  filtering with deterministic scalar coefficient design
- `tremolo`: sinusoidal amplitude modulation from `1 - depth / 100` to `1`
- `overdrive`: SoX-ng's cubic soft-clipping drive with `color / 200` bias and
  a stateful high-pass output blend
- `saturation`: SoX-ng's tanh, sqrt, or diode nonlinear transfer, wet/dry
  blend, asymmetric offset, and safety gain compensation
- `repeat`: finite output count and planar channel grouping
- `remix`: channel out-spec routing, silent channels, default-scaled
  multi-input mixdown, source gain modifiers, and level-scaling options
- `centercut`: overlapping spectral center estimation that emits left
  residual, right residual, and center channels from stereo input
- `oops`: left-minus-right stereo extraction duplicated to both output
  channels, with mono input rejected and extra channels ignored
- `swap`: adjacent channel pairs exchange positions, with odd trailing channels
  preserved
- `dcshift`: add a constant normalized full-scale offset; the effect itself
  does not clip unless SoX-ng's optional limiter gain is configured, while
  PCM16 WAV output clips plain shifted samples to the representable range
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
- `pad 0` is identity and `pad length@position` preserves surrounding frames
- `vol 1` is identity
- `norm` preserves silence and finite bounded input stays finite
- `contrast` preserves finite bounded input as finite output
- `softvol` default settings preserve bounded input and configured recovery
  stays finite
- `tremolo 0` is identity and finite bounded input stays finite
- `overdrive` preserves finite bounded input as finite output
- `saturation` preserves finite bounded input as finite output
- `repeat 0` is identity and finite bounded input stays finite
- `remix` identity routing preserves samples and finite bounded input stays
  finite
- `silence 0` is identity and finite bounded input stays finite
- `centercut` finite stereo input emits finite three-channel output and rejects
  non-stereo input
- `oops` finite multichannel input stays finite and emits stereo output
- `swap` twice returns the original signal and finite bounded input stays finite
- `channels` with the current channel count is identity and channel-converted
  finite input stays finite
- `gain +6 dB` followed by `gain -6 dB` approximately returns the original signal within tolerance

### L5: chunk invariance

Every streaming-capable effect should behave the same whether data is processed whole or in chunks.

Required chunk sizes:

```text
1, 2, 7, 15, 16, 17, 31, 32, 33, 64, 255, 1024, random seeded chunks
```

The Rust helper `auralis_testkit::chunk_invariance` is the shared source for
that matrix. It injects empty chunks and a final empty call for flush-path
coverage, and it reports the deterministic random seed in schedule labels.
Current L5 integration tests cover `Gain`, `DcShift`, `Fade`, `Vol`, `Tremolo`,
stateful `Overdrive`, and streaming-safe `EffectChain` execution. `Norm` is documented as a
whole-buffer scan, `Repeat`, `Remix`, and `Centercut` are documented as
whole-buffer structural or spectral transforms, and `Channels` is documented as
an explicit shape-changing structural transform; these are not chunk-invariant
under the current API.

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
The Rust testkit provides `backend_conformance` helpers that run one case under
forced scalar and requested SIMD backend selection, then report the case ID,
backend labels, first failing index, and error metrics for exact or
tolerance-based comparisons.

### L7: fuzzing, sanitizers, and coverage

Rust reduces many memory risks, but fuzzing is still required for:

- WAV parser behavior
- pipeline parser behavior
- effect argument parsing
- edge-case chunking
- unsupported format rejection

Coverage gates should focus on touched code and DSP modules, not a misleading whole-repository percentage.

The L7 baseline is checked in as two layers:

- `crates/auralis-fuzz-targets` provides stable-Rust fuzz target drivers and
  deterministic smoke tests.
- `fuzz/` provides cargo-fuzz-compatible wrappers and seed corpus files for
  local libFuzzer runs.

Current fuzz target names are:

```text
wav_parser
unsupported_wav_format
effect_command
effects_file
golden_manifest
```

Run the stable smoke check after changing parser, codec, or manifest
boundaries:

```bash
cargo test -p auralis-fuzz-targets --all-features
cargo check --manifest-path fuzz/Cargo.toml --bins
cargo run --manifest-path fuzz/Cargo.toml --bin effect_command -- \
  fuzz/corpus/effect_command -runs=1
```

Run local libFuzzer campaigns with `cargo-fuzz` installed:

```bash
cargo +nightly fuzz run wav_parser -- -max_total_time=60
cargo +nightly fuzz run unsupported_wav_format -- -max_total_time=60
cargo +nightly fuzz run effect_command -- -max_total_time=60
cargo +nightly fuzz run effects_file -- -max_total_time=60
cargo +nightly fuzz run golden_manifest -- -max_total_time=60
```

Cargo-fuzz writes reproducible crash inputs under `fuzz/artifacts/<target>/`
and can minimize them with:

```bash
cargo +nightly fuzz tmin <target> fuzz/artifacts/<target>/<crash-file>
```

For Linux sanitizer checks, use nightly Rust because sanitizer flags are still
unstable:

```bash
RUSTFLAGS="-Z sanitizer=address" \
  cargo +nightly test -Z build-std --target x86_64-unknown-linux-gnu \
  -p auralis-wav -p auralis-effects -p auralis-testkit -p auralis-fuzz-targets
```

For touched-module coverage, use `cargo-llvm-cov` and keep the package list
focused on modules affected by the change:

```bash
cargo llvm-cov --workspace --all-features \
  -p auralis-wav -p auralis-effects -p auralis-testkit -p auralis-fuzz-targets \
  --lcov --output-path target/llvm-cov/l7-touched.info
```

When coverage is part of the feature acceptance criteria, keep the generated
summary or LCOV path in the validation notes. The repository does not require a
global percentage gate yet; touched-package coverage reports are the expected
first step.

Coverage report artifacts are a later acceptance requirement for milestones
that change shared behavior, parser surfaces, or DSP modules. Until that gate
is wired into CI, validation notes should name the generated summary or LCOV
artifact explicitly when coverage was required.

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

The implemented Rust metric helpers are `max_abs_error`, `rms_error`, `snr_db`,
`peak`, and `dc_offset`. Empty inputs are treated as silence, unequal error
metric inputs compare missing samples as `0.0`, and NaN inputs return NaN.

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
