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

Auralis is currently pre-alpha. The repository contains the initial Rust
workspace skeleton, uv-based Python test harness, and core audio type
vocabulary with planar internal audio buffers. The codec trait boundary is in
place for WAV-only scope and explicit unsupported-format reporting. PCM16 WAV
decoding into planar `f32` buffers and encoding back to PCM16 WAV are
implemented, the `auralis inspect` CLI reports PCM16 WAV metadata, and
`auralis run input.wav output.wav` performs a decode-through-buffer copy
pipeline and can apply constant gain with `--gain-db <DB>`, an end-exclusive
trim with frame or seconds ranges, zero padding with frame counts, frame-level
reversal with `--reverse`, constant DC offset with `--dc-shift <SHIFT>`, or
linear fades with `--fade-in-frame <FRAMES>` and `--fade-out-frame <FRAMES>`.
The scalar `gain`, `dcshift`, and `fade` DSP kernels, the typed `Gain`, `DcShift`,
`Trim`, `Pad`, `Reverse`, and `Fade` effect processors, the high-level library
chain API for applying gain, dcshift, trim, pad, reverse, and fade, and the CLI
gain/dcshift/trim/pad/reverse/fade transforms are implemented. The Rust
effects crate also exposes a deterministic name registry and typed command
parser for the implemented effect subset; supported names and aliases resolve
to typed descriptors, parsed command tokens become typed effect configs, and
unknown names or unsupported SoX-ng options return stable diagnostics. Parsed
commands can be grouped into an in-memory `EffectChain` and executed in order
with indexed command-context errors and forced scalar/SIMD backend selection.
The chain path supports SoX-ng-style `gain -h` and `gain -r` headroom metadata,
`gain -n` peak normalization, `gain -l` limiting, and channel-aware `gain -e`,
`gain -B`, and `gain -b` scans: `gain -h DB` applies the fixed attenuation and
records reclaimable headroom, and a later `gain -r` restores as much as
possible without clipping. `auralis run <input.wav> <output.wav> gain -3
dcshift 0.125 reverse` exposes the same typed chain model at the CLI,
preserving positional user order while the earlier single-effect flags remain
available for compatibility. The golden
suite now includes standalone effect coverage in `tests/golden/effects.toml`
plus a `tests/golden/chains.toml` manifest for representative editing, level,
gain headroom/reclaim, and fade/gain filter-style positional chains against
SoX-ng.
The Rust testkit includes deterministic sample comparison metrics for max absolute
error, RMS error, SNR, peak, and DC offset. The uv-based Python testkit exposes
shared corpus, metric, and SoX-ng wrapper helpers for cross-language golden
tests. The Rust testkit also parses TOML golden manifests, renders stable
Auralis and SoX-ng command vectors for reproducible comparison reports, and
provides backend conformance helpers for exact and tolerance-based
scalar-vs-SIMD differential tests. The effects crate also parses
SoX-ng-inspired effects files into typed `EffectChain` values with blank-line,
comment, quote, escape, and line/column diagnostic handling, and
`auralis run --effects-file <FILE>` executes those chains through the same
ordered pipeline as positional CLI effect chains. Positional chains and effects
files preserve explicit `:` chain boundaries for deterministic rendering and
diagnostics, while unsupported `newfile` and `restart` boundary controls return
stable not-yet-implemented errors. The high-level facade and CLI also support
SoX-ng-style concatenate, sequence, mix, mix-power, merge, and multiply input
combiners. In `auralis run first.wav out.wav --combine concatenate --input second.wav`,
decoded inputs are appended before effects run and must have matching sample
rates and channel counts. `--combine sequence` uses the same serial playback
order for boundaries that can be represented in one output WAV, and reports a
clear boundary error if the sample rate or channel count changes. `--combine
mix` scales each input by `1 / input_count`, sums corresponding channels, uses
the longest input length and largest channel count, treats missing frames or
channels as silence, and leaves any out-of-range mixed samples to be clipped by
the output encoder. `--combine mix-power` uses the same output-shape and
silence rules but scales each input by `1 / sqrt(input_count)` for equal-power
mixing. `--combine merge` creates one multichannel output containing every
channel from every input in caller order, uses the longest input length, and
fills shorter input tails with silence. Merge is a structural copy, so SIMD is
not applicable. `--combine multiply` multiplies corresponding channels and
samples from every input, uses the longest input length and largest channel
count, treats missing frames or channels as silence, and uses the
backend-dispatched scalar/SIMD multiply kernel. `auralis run --channels N`
uses an explicit output policy that mirrors SoX-ng's output `--channels`
shorthand: the CLI preserves the pipeline channel count by default, converts
only when a target count is requested, and `--no-auto-channels` turns that
conversion into a strict channel-count check for tests. `auralis run --rate N`
uses the same explicit output-boundary policy for sample rate: the CLI
preserves the pipeline rate by default, applies Auralis' deterministic scalar
linear resampler only when requested, and `--no-auto-rate` turns the request
into a strict sample-rate check. The fuller user-visible `rate` effect and
quality modes remain future DEVELOPMENT.md work. `auralis run --guard` applies
an explicit final output-level policy that attenuates only when the processed
buffer would exceed full scale before PCM16 encoding, while `--norm[=DB]`
normalizes non-silent output to a requested peak level, defaulting to 0 dBFS.
The high-level library exposes the same behavior through `OutputLevelPolicy`,
`Pipeline::with_output_guard`, and `Pipeline::with_output_normalization`;
`Pipeline::into_audio_buffer` remains free of output-boundary level changes.
The SIMD crate defines the
Auralis-owned backend trait skeleton with a scalar reference backend,
deterministic `scalar` / `simd` backend selection, scalar/SIMD PCM16/`f32`
sample conversion in both directions, and backend-dispatched linear
`gain_f32`, `dc_shift_f32`, `fade_f32`, `mix_f32`, and `multiply_f32` kernels.
L4 property and metamorphic coverage now uses Rust-side `proptest` checks for
the implemented effect set, including identity parameters, reverse-twice
invariance, gain/inverse-gain round trips for non-clipping input, and
finite-output behavior for bounded finite samples. L5 chunk-invariance coverage
uses a shared Rust testkit matrix over fixed chunk sizes and deterministic
seeded random chunks for `Gain`, `DcShift`, `Fade`, and streaming-safe chains.
Source modularization debt is scoped by a checked-in file-size audit and module
ownership map in `doc/development/05-source-module-map.md`. The high-level
`auralis` facade has been split into ownership modules while keeping its public
re-exports stable, and `auralis-simd` now keeps backend metadata, selection,
conversion, arithmetic kernels, and focused conformance tests in separate
modules. `auralis-effects` has also been split into per-effect implementation
modules with focused unit tests while preserving the existing public effect
types and command/chain behavior. `auralis-wav` now keeps PCM16 reader,
writer, format validation, sample-conversion glue, and focused integration
tests in separate ownership modules. The remaining oversized CLI and testkit
files have been split, and `python3 tools/check_rust_source_lines.py` enforces
the no-thousand-line Rust source policy locally.
Other effect transform CLI options are still intentionally unimplemented.

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
        .dc_shift(0.125)
        .trim_seconds(0.0, 10.0)
        .fade_frames(0, 12_000)
        .write_wav("output.wav")?;

    Ok(())
}
```

The first command-line equivalent should be simple and scriptable:

```bash
auralis run input.wav output.wav --gain-db -3
```

The positional effect-chain form uses the same typed command parser and chain
executor as the library:

```bash
auralis run input.wav output.wav gain -3 dcshift 0.125 reverse
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
│   ├── auralis/
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
- `SampleRate`
- `ChannelCount`
- `SampleFormat`
- `FrameCount`
- `TimeSeconds`
- `Hertz`
- `Decibels`
- `AuralisError`
- `Result<T>`

This crate should have no dependency on CLI, WAV libraries, Python, or SIMD backends.

### `auralis`

Provides the high-level library facade:

- `AudioFile::open_wav`
- `AudioFile::open_wavs_concatenated`
- `AudioFile::open_wavs_sequenced`
- `AudioFile::open_wavs_mixed`
- `AudioFile::open_wavs_mix_powered`
- `AudioFile::open_wavs_merged`
- `AudioFile::open_wavs_multiplied`
- `concatenate_audio_buffers`
- `sequence_audio_buffers`
- `mix_audio_buffers`
- `mix_power_audio_buffers`
- `merge_audio_buffers`
- `multiply_audio_buffers`
- `AudioFile::into_pipeline`
- `Pipeline::gain_db`
- `Pipeline::dc_shift`
- `Pipeline::trim_frames`
- `Pipeline::trim_seconds`
- `Pipeline::pad_frames`
- `Pipeline::fade_frames`
- `Pipeline::reverse`
- `Pipeline::apply_effect_chain`
- `Pipeline::write_wav`

This crate wires together core buffers, WAV I/O, and typed effects while keeping
the lower-level crates available for focused testing and specialized use.

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

The crate root is a small facade over focused modules:

- `reader`: PCM16 stream/path decode helpers and `Pcm16WavReader`;
- `writer`: PCM16 stream/path encode helpers and `Pcm16WavWriter`;
- `format`: WAV sample encoding, PCM16 validation, hound spec construction,
  frame counts, and malformed-input mapping;
- `sample_conversion`: backend-dispatched PCM16/`f32` conversion glue;
- `error`: typed `WavError`, result alias, and codec-error conversion.

Initial WAV scope:

- PCM 16-bit little-endian.
- PCM 24-bit little-endian if practical in the first pass; otherwise reserve the API and explicitly return `UnsupportedFormat`.
- IEEE float 32-bit if practical; otherwise reserve the API and explicitly return `UnsupportedFormat`.
- Mono and stereo first; multi-channel support should be designed but may be gated behind tests.

The WAV module should expose decoded planar `f32` buffers to the rest of the system.
PCM16 decode uses the Auralis sample-conversion backend boundary, with an
explicit backend entry point for scalar-vs-SIMD decode validation.

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
`gain`, `dcshift`, and linear `fade` have explicit backend-dispatched entry
points that keep scalar as the default and use the Auralis SIMD backend only
when requested.

### `auralis-effects`

Contains typed effect processors built from DSP primitives:

- `Gain`
- `DcShift`
- `Trim`
- `Pad`
- `Reverse`
- `Fade`
- later: `Remix`
- later: `Lowpass`, `Highpass`, `Biquad`, `Rate`, `Compand`, `Delay`, `Reverb`, `Silence`

Effect implementations should be block-based and streaming-aware from the beginning, even if the initial CLI processes whole files.
The crate root is a small facade; effect-local behavior lives in focused
`gain`, `dcshift`, `trim`, `pad`, `reverse`, and `fade` modules, with shared
typed errors in `error`.
The crate also owns the static effect registry and typed command parser used by
upcoming chain parsing. Implemented SoX-ng names such as `gain`, `dcshift`,
`trim`, `pad`, `reverse`, and `fade` resolve to typed descriptors; aliases such
as `dc-shift` and `gain-db` resolve to their canonical names; unknown names
receive deterministic suggestions; and known SoX-ng effects without Auralis
coverage return a stable missing-coverage diagnostic. Tokenized commands such
as `["gain", "-3"]`, `["trim", "48000", "96000"]`, and `["fade", "t",
"24000", "0", "24000"]` parse into typed `EffectCommand` variants. The parser
accepts the frame-count subset implemented by Auralis and supports SoX-ng fade
curve tokens `q`, `h`, `l`, `t`, and `p`, stop-position fade-out semantics,
and fade-out lengths measured backward from the stop position. The `dcshift`
command accepts SoX-ng's optional limiter gain argument, for example
`dcshift 0.5 0.05`, and applies immediate SoX-ng-style clipping only on that
limiter path. The implemented `gain` command
forms include plain fixed gain, `gain -h`, `gain -r`, combined
`gain -rh`/`gain -hr` headroom reclaim, peak normalization with `gain -n`, and
the simple limiter with `gain -l`, plus channel peak equalization with
`gain -e`, RMS balancing with `gain -B`, and RMS balancing with clip protection
through `gain -b`.
Parsed `EffectCommand` values render back to canonical SoX-ng-style token
vectors using stable effect names, explicit default arguments, and deterministic
numeric formatting, so equivalent values such as `gain`, `gain 0`, and
`gain-db 0.0` produce the same manifest representation. `EffectChain` groups
typed commands into an in-memory sequential chain, preserves explicit `:`
boundaries between chain segments, applies implemented commands in caller
order, supports forced scalar/SIMD backend selection for backend-aware effects,
and reports processing failures with the zero-based command index, canonical
command tokens, failed argument family, and typed source error. Flat token
streams can also be parsed into an `EffectChain`, which is how `auralis run
<input> <output> gain -3 : reverse` shares the same ordering and diagnostics as
the library API. The `newfile` and `restart` controls are reserved for later
multi-output/restart features and currently return stable diagnostics.

Effects files use the same token and command model as positional chains. A `#`
outside quotes starts a comment, blank lines are ignored, single and double
quotes group whitespace into one token, and a backslash escapes the next
character outside single quotes. Each non-comment line may contain one or more
commands:

```text
# level and editing chain
gain -3
dcshift 0.125 : reverse
fade t 24000 0 24000
```

Parsing this text with `parse_effects_file_str` or `parse_effects_file`
produces the same typed `EffectChain` as the flat CLI-style token stream `gain
-3 dcshift 0.125 : reverse fade t 24000 0 24000`. The CLI accepts the same file
with `auralis run input.wav output.wav --effects-file chain.effects`; effects
files are mutually exclusive with positional chain tokens and legacy effect
flags because their relative order would otherwise be ambiguous. Empty boundary
segments, unsupported `newfile`/`restart` controls, malformed quotes, dangling
escapes, unknown effects, unsupported SoX-ng effects, invalid command
arguments, missing files, and unreadable files report stable errors.

### `auralis-simd`

Contains optional SIMD acceleration. This is a backend layer, not part of the
high-level public API. It owns the backend trait skeleton, backend descriptors,
deterministic named backend selection, the scalar reference backend marker, and
PCM16/`f32` conversion kernels in both directions plus the linear `gain_f32`,
`dc_shift_f32`, `fade_f32`, `mix_f32`, and `multiply_f32` kernels. The scalar
kernels are the exact reference implementations; the SIMD kernels use
`rten-simd` behind the `simd` feature and fall back through Auralis backend
selection when SIMD is unavailable. The crate root is a facade over focused
backend, selection, conversion, and per-kernel modules.

Backend names are stable lowercase strings:

- `scalar`
- `simd`

Requesting `scalar` always selects the scalar reference backend. Requesting
`simd` selects SIMD only when the `simd` Cargo feature is enabled and the active
target supports Auralis' SIMD backend. Otherwise selection falls back to
`scalar` and reports whether the fallback happened because the feature is
disabled or because the target is unsupported.

Selected future SIMD abstraction:

```toml
rten-simd = { version = "0.24.0", optional = true }
```

Reasons:

- stable Rust
- portable SIMD abstraction
- runtime ISA dispatch
- supports AVX2, AVX-512, Arm Neon, and WebAssembly SIMD
- suitable for custom 1D slice kernels

`rten-simd` must not leak into public API types. Auralis defines its own backend
traits and keeps SIMD as an implementation detail behind the `simd` feature.

Initial SIMD targets:

- `gain_f32`
- `dc_shift_f32`
- `fade_f32`
- `mix_f32`
- `clip_f32`
- `i16_to_f32`
- `f32_to_i16`
- later: FIR and polyphase resampling inner loops

`i16_to_f32` maps PCM16 samples by dividing by `32768.0`. `f32_to_i16`
requires finite input samples, clips to `[-1.0, 1.0]`, scales by `32768.0`,
rounds halfway cases away from zero, and clips the final integer to the PCM16
range. WAV decode and encode expose explicit backend hooks for scalar-vs-SIMD
conformance tests while preserving scalar defaults. `gain_f32` accepts a linear
amplitude multiplier from the DSP `gain` decibel conversion, preserves signed
zero for infinite-gain edge cases, and is covered by scalar-vs-SIMD parity
tests for deterministic fixtures, tails, near-clipping values, silence, NaN,
and infinity behavior. `dc_shift_f32` accepts a normalized full-scale offset
from the typed `dcshift` effect and is covered by scalar-vs-SIMD parity tests
for deterministic fixtures, tail lengths, silence, denormals, near-clipping
values, NaN, and infinity behavior. `fade_f32` applies linear fade-in and
fade-out envelope multiplication over frame-indexed channel segments and is
covered by scalar-vs-SIMD parity tests for fade-in, fade-out, combined fades,
tail lengths, and full CLI output.

Do not prioritize SIMD for state-machine-heavy or recursive algorithms at first.
Do not implement SIMD before scalar correctness tests and differential tests
exist.

### `auralis-cli`

Provides the `auralis` binary.

Initial CLI goals:

```bash
auralis --version
auralis inspect input.wav
auralis run input.wav output.wav
auralis run input.wav output.wav --gain-db -3
auralis run input.wav output.wav --backend simd --gain-db -3
auralis run input.wav output.wav --dc-shift 0.125
auralis run input.wav output.wav --backend simd --dc-shift 0.125
auralis run input.wav output.wav --trim-start-frame 48000 --trim-end-frame 96000
auralis run input.wav output.wav --trim-start-seconds 1.0 --trim-end-seconds 2.0
auralis run input.wav output.wav --pad-start-frame 24000 --pad-end-frame 48000
auralis run input.wav output.wav --fade-in-frame 24000 --fade-out-frame 24000
auralis run input.wav output.wav --backend simd --fade-in-frame 24000 --fade-out-frame 24000
auralis run input.wav output.wav --reverse
auralis run input.wav output.wav gain -3 dcshift 0.125 reverse
auralis run input.wav output.wav gain -3 : dcshift 0.125 reverse
auralis run input.wav output.wav --backend simd gain -3 fade t 24000 0 24000
auralis run input.wav output.wav --effects-file chain.effects
auralis run first.wav output.wav --combine concatenate --input second.wav reverse
auralis run first.wav output.wav --combine sequence --input second.wav reverse
auralis run first.wav output.wav --combine mix --input second.wav reverse
auralis run first.wav output.wav --combine mix-power --input second.wav reverse
auralis run first.wav output.wav --combine merge --input second.wav reverse
auralis run first.wav output.wav --combine multiply --input second.wav reverse
auralis run stereo.wav mono.wav --channels 1
auralis run pipeline.toml
auralis completions zsh
```

The positional effect chain starts after the input and output paths. A `:`
token preserves an explicit chain boundary in the parsed representation and
deterministic rendering; current processing still executes the implemented
commands sequentially in memory. SoX-ng `newfile` and `restart` boundary
controls are recognized but rejected until their own pipeline features exist.
Backend selection remains an option, but legacy one-effect flags such as
`--gain-db` are not combined with positional chain tokens or `--effects-file`
because their relative order would be ambiguous. Effects files use the same
parser as the library `parse_effects_file` API.

Input combiners run before the effect chain. The first input is the
existing positional input, and each additional input is supplied with
`--input <FILE>` while `--combine <METHOD>` records the combiner method.
`concatenate` accepts inputs with different frame lengths, appends them in
caller order, and rejects mismatched sample rates or channel counts before any
effects are applied. `sequence` also appends representable boundaries in caller
order; because Auralis currently produces one output buffer and one PCM16 WAV
file, a sequence boundary that changes sample rate or channel count is rejected
instead of reopening the output stream as SoX-ng may do for some devices. `mix`
is a parallel combiner: it requires matching sample rates, scales every input
by `1 / input_count`, sums corresponding channels, and emits the longest input
length with the maximum channel count. Shorter inputs and missing channels are
silence. The mix kernel does not clip; PCM16 WAV writing clips out-of-range
samples using the normal encoder rules. `mix-power` keeps the same parallel
output shape and silence behavior but uses SoX-ng-style equal-power balancing
with `1 / sqrt(input_count)`. This can leave summed samples outside
`[-1.0, 1.0]` more often than `mix`; the in-memory combiner does not clip them,
and PCM16 WAV writing clips out-of-range samples using the normal encoder
rules. `merge` is also parallel, but structural: it requires matching sample
rates, emits the longest input length, and sets the output channel count to the
sum of every input channel count. Channels are ordered by input, so two mono
files become a stereo output, and shorter inputs contribute silent tail frames.
Merge has no arithmetic kernel, so SIMD is documented as not applicable.
`multiply` is parallel and arithmetic: it requires matching sample rates,
multiplies corresponding input samples without balancing, emits the longest
input length with the maximum channel count, and treats shorter inputs or
missing channels as silence so any missing contribution makes that output
sample zero. A single input is an identity copy. The multiply combiner does not
clip in memory; PCM16 WAV writing clips out-of-range samples using the normal
encoder rules.

Output channel conversion is an explicit output-boundary policy, not hidden
library behavior. The high-level library defaults to preserving the current
pipeline channel count. Callers can use `Pipeline::with_output_channels` or
`Pipeline::with_channel_conversion_policy(ChannelConversionPolicy::Automatic(_))`
to request SoX-ng-style `channels` conversion before writing; downmixing
averages the same deterministic channel groups as SoX-ng and upmixing
duplicates channels round-robin. Tests and strict callers can use
`ChannelConversionPolicy::Require(_)`, exposed in the CLI as
`--channels N --no-auto-channels`, to fail if conversion would have been
inserted.

Output sample-rate conversion follows the same explicit output-boundary model.
The high-level library defaults to preserving the current pipeline sample rate,
and `Pipeline::into_audio_buffer` never applies output-rate policy. Callers can
use `Pipeline::with_output_sample_rate` or
`Pipeline::with_sample_rate_conversion_policy(SampleRateConversionPolicy::Automatic(_))`
to request deterministic scalar conversion before writing. The converter emits
`round(input_frames * target_rate / source_rate)` frames and samples each
channel with linear interpolation at `output_frame * source_rate / target_rate`.
Tests and strict callers can use `SampleRateConversionPolicy::Require(_)`,
exposed in the CLI as `--rate N --no-auto-rate`, to fail if conversion would
have been inserted. SIMD is currently not used for this boundary resampler; the
later `rate` effect features own the broader quality and optimization work.

Output level control is explicit as well. The high-level library defaults to
preserving sample levels, so PCM16 writing performs the same documented clipping
as before. Callers can use `Pipeline::with_output_guard` or
`Pipeline::with_output_level_policy(OutputLevelPolicy::Guard)` to attenuate the
final buffer only when its absolute peak exceeds full scale. They can use
`Pipeline::with_output_normalization` or
`OutputLevelPolicy::Normalize(Decibels)` to scale non-silent output to a target
peak before writing. The CLI exposes these policies as `--guard` and
`--norm[=DB]`; `--norm` defaults to 0 dBFS, and `--guard` cannot be combined
with `--norm`. Automatic dither insertion has been deferred to the post-`dither`
effect plan in DEVELOPMENT, so current Auralis output never adds hidden dither
noise.

Selected crates:

- `clap` for the parser.
- `clap_complete` for shell completions.
- `clap_mangen` for generated man pages.

### `auralis-testkit`

Contains test utilities shared by Rust tests and Python tests:

- shared deterministic corpus generation by stable ID for silence, impulse,
  step, sine, sweep, seeded noise, full-scale, near-zero, odd-length,
  mono/stereo, and short-buffer cases
- raw f32 helpers
- WAV decode helpers
- metric calculation for max absolute error, RMS error, SNR, peak, and DC offset
- golden test manifest handling, including standalone effect coverage,
  output-channel and output-rate metadata for cases where SoX-ng auto-inserts
  `channels` or `rate` conversion, plus output-level guard and normalization
  comparison manifests
- scalar-vs-SIMD backend conformance helpers
- SoX-ng command wrapper
- tolerance definitions
- shared `auralis.golden.failure.v1` failure artifact generation for Rust and
  Python golden runners

Golden manifests use TOML tables keyed under `id`:

```toml
[id.gain_minus_3_mono]
input = "sine_48k_mono.wav"
corpus_id = "l0/sine_mono_32"
auralis = ["--gain-db", "-3"]
sox_ng = ["gain", "-3"]
max_abs = 1e-4
rms = 1e-6
snr_db = 90.0
```

The `auralis` array is appended after `auralis run <input> <output>`, while
`sox_ng` is appended after `sox_ng -R -D <input> <output>`. Case IDs, tolerance
fields, and command arguments are validated before tests run so failure reports
can rely on deterministic command rendering. The testkit also renders command
vectors as display strings for reports with stable double-quote escaping for
spaces, quotes, backslashes, and control characters while keeping the original
argument vectors available for process execution. Explicit chain boundary
tokens render deterministically as `:` in both Auralis and SoX-ng command
displays. A single-input case may set `corpus_id = "..."`, and multi-input
cases may set `corpus_ids = ["...", "..."]`, so Rust and Python golden runners
can generate the same deterministic PCM16 fixtures without per-test local
sample builders. A case may set `output_channels = N`; if it also sets
`sox_ng_auto_channels = true`, the manifest records that SoX-ng is expected to
auto-insert its `channels` effect from the output option. A case may also set
`output_sample_rate = N` and `sox_ng_auto_rate = true` to record SoX-ng's
automatic `rate` insertion from an output sample-rate option.

The root `tests/golden/chains.toml` manifest records positional-chain coverage
for a structural editing chain, a level-processing chain, gain
headroom/reclaim, and the currently implemented fade/gain filter-style chain.
`tests/golden/concat.toml`,
`tests/golden/sequence.toml`, `tests/golden/mix.toml`,
`tests/golden/mix_power.toml`, `tests/golden/merge.toml`, and
`tests/golden/multiply.toml` record combiner coverage for mismatched mono input
lengths and stereo combine-before-reverse chains.
`tests/golden/effects.toml` records standalone mono and stereo SoX-ng coverage
for each implemented effect: `gain`, `dcshift`, `trim`, `pad`, `reverse`, and
`fade`, including standalone `gain -h`, `gain -n`, and `gain -l` cases for
headroom attenuation, peak normalization, and limiting, stereo `gain -e`,
`gain -B`, and `gain -b` cases for channel equalization and balancing, and
fade-in cases for the SoX-ng `q`, `h`, `l`, `t`, and `p` curve families plus
linear fade-out-at-end and explicit stop-position fade-out cases.
Those standalone effect cases isolate effect behavior: output rate/channel
conversion is absent, guard and norm are absent, and SoX-ng automatic dithering
is disabled by the runner's `-D` flag.
`tests/golden/auto_channels.toml` records output-channel policy coverage where
SoX-ng auto-inserts `channels` conversion, and `tests/golden/auto_rate.toml`
records output-rate policy coverage where SoX-ng auto-inserts `rate`
conversion. `tests/golden/auto_level.toml` records output-level guard and
normalization coverage for representative clipping and peak-normalization
cases. The Python golden runners
resolve each manifest `corpus_id` or `corpus_ids`, generate deterministic PCM16
fixtures, execute both command lines, compare decoded sample metadata plus
max-abs/RMS/SNR/peak metrics, and write a JSON failure report when output drifts
outside its manifest tolerance. Failure reports use the shared
`auralis.golden.failure.v1` schema and include the case ID, backend, Auralis and
SoX-ng versions, manifest input and corpus IDs, Auralis and SoX-ng command
vectors, metric thresholds, decoded output sample rate/channel/frame metadata,
measured metrics, and structured failing-metric entries with expected value,
actual value, comparison direction, and first offending flattened sample index
where the metric is sample-local.

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

Core rule:

> Public APIs do not depend on concrete implementation crates. Third-party
> crates belong at boundary layers, test layers, CLI layers, or replaceable
> backend layers.

Users of `auralis-core` should not see names such as `hound`, `clap`,
`rten-simd`, `rubato`, `pyo3`, or `ndarray` in public API types.

### Add in the first workspace pass

| Area | Choice | Boundary |
|---|---|---|
| CLI | `clap`, `clap_complete`, `clap_mangen` | `auralis-cli`; generated help/completions/man pages from one definition |
| Errors | `thiserror`, `miette`, small amounts of `anyhow` | `thiserror` for typed library errors; `miette` and `anyhow` stay in binaries, tests, and glue |
| Config and reports | `serde`, `toml`, `serde_json` | pipeline manifests are TOML; machine-readable reports are JSON |
| WAV | `hound` | wrapped inside the WAV codec crate; never exposed by `auralis-core` |
| Observability | `tracing`, `tracing-subscriber` | libraries emit structured events; CLI initializes subscribers |
| Float assertions | `approx` | dev/test only |
| Property tests | `proptest` | dev/test only |
| CLI tests | `assert_cmd`, `predicates`, `tempfile` | dev/test only |
| Snapshots | `insta` with `serde` | dev/test only; for help, diagnostics, manifests, and reports |
| Benchmarks | `criterion` | dev/bench only |

Library APIs must return typed errors, not `anyhow::Result<T>`. `miette` is for
diagnostic presentation at the CLI boundary. `hound` handles initial WAV I/O,
but Auralis tests compare decoded PCM and metadata rather than whole WAV bytes
unless a test is specifically about serialization.

Future format support follows a pure Rust policy unless DEVELOPMENT explicitly
changes that policy. Codec adapters may depend on audited pure Rust crates
behind feature gates, but the current roadmap does not plan external `ffmpeg`
command backends, `ffmpeg-next`, libFLAC wrappers, LAME wrappers, libvorbis
wrappers, `libopusenc`, FDK-AAC, or other native codec-library bindings.

### Selected direction, but optional or later

| Area | Choice | Rule |
|---|---|---|
| SIMD | `rten-simd` behind `simd` | optional backend only; scalar remains the reference |
| Frequency-domain tests | `realfft`, `rustfft` | add to testkit/dev dependencies when spectral tests begin |
| Batch parallelism | `rayon` behind `parallel` | for many files, test cases, stems, or render jobs; not the initial single-stream effect chain |
| Byte casting | `bytemuck` behind `pod` | only after normal parsing is correct and profiling justifies it |
| Small allocation optimization | `smallvec` behind `smallvec` | only for proven small-vector pressure |
| Python package | `pyo3`, `maturin`, `numpy` | future `auralis-py`; keep Rust API and buffer model ready |

### Do not introduce now

| Crate | Decision |
|---|---|
| `rubato` | Do not make it the core resampler. Later it may be a reference or benchmark target against Auralis scalar rate and SoX-ng golden tests. |
| `symphonia` | Do not add until the WAV-only milestone is stable and multi-format decoding is actually in scope. |
| `ffmpeg` / `ffmpeg-next` | Not planned under the current pure Rust codec policy. |
| `ndarray` | Do not use in `auralis-core` public APIs. Keep the core buffer as planar `Vec<f32>` and convert at Python/test boundaries later. |
| `serde_yaml` | Do not use. Configuration is TOML; machine reports are JSON. |
| `tokio` | Do not use in the initial offline CPU-bound DSP phase. Use synchronous file I/O and add batch parallelism later via Rayon if needed. |

Minimal first-pass workspace dependencies:

```toml
[workspace.dependencies]
thiserror = "2"
serde = { version = "1", features = ["derive"] }
toml = "1"
serde_json = "1"
hound = "3"
clap = { version = "4", features = ["derive", "wrap_help"] }
clap_complete = "4"
clap_mangen = "0.3"
miette = { version = "7", features = ["fancy"] }
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Shared dev/test/bench dependency versions. Individual crates reference these
# from their own [dev-dependencies] with `workspace = true`.
approx = "0.5"
proptest = "1"
assert_cmd = "2"
predicates = "3"
tempfile = "3"
insta = { version = "1", features = ["serde"] }
criterion = "0.8"
```

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

The checked-in coverage gate is
`doc/development/05-layered-coverage.toml`. It records the L0-L7 status, linked
tests, and N/A reason for each implemented effect and pipeline primitive. Run
`python3 tools/check_layered_coverage.py` after changing effect, combiner,
policy, parser, or test coverage so future feature work cannot drift away from
the matrix.

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
- `pad 0` is identity
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
Current L5 integration tests cover `Gain`, `DcShift`, `Fade`, and
streaming-safe `EffectChain` execution.

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
