# Auralis Architecture Notes

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
auralis render input.wav -o output.wav --fx 'gain -3'
```

The render command uses the same typed command parser and chain
executor as the library:

```bash
auralis render input.wav -o output.wav --fx 'gain -3' --fx 'dcshift 0.125' --fx reverse
```

A structured pipeline form should also exist for reproducible batch workflows:

```bash
auralis run Auralis.toml
```

Example `Auralis.toml`:

```toml
version = "auralis.graph/v1"

[[sources]]
id = "input"
path = "input.wav"

[[chains]]
id = "main"
input = "input.audio"
steps = [
  { op = "gain", by = "-3dB" },
  { op = "trim", range = "0s..10s" },
  { op = "fade", fade_out = "0.25s" },
]

[[sinks]]
id = "output"
input = "main.audio"
path = "output.wav"
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
- Whole-buffer behavior and any internal state assumptions.
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

Defines codec traits, format-neutral interfaces, and the facade over backend
decoders plus the narrow planned encode paths:

- `AudioReader`
- `AudioWriter`
- `CodecRegistry`
- `DecodedAudio`
- `EncodedAudioFormat`

It should include the file kinds that the facade can dispatch:

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

The target decode path uses Symphonia first. WAV also starts with Symphonia;
`hound` is only a fallback or special-case WAV path when Symphonia cannot handle
an Auralis-supported case. Symphonia is decode-only. The target encode path
starts with WAV; FLAC encode may be planned through `flacenc`; other non-WAV
encode entries are placeholders until a separate encoder policy exists.
Existing per-format codec crates are migration debt to consolidate behind this
facade, not the desired architecture.

### `auralis-dsp`

Contains scalar DSP primitives and reference implementations:

- gain
- dc shift
- clipping
- fades
- channel mixing
- FIR primitives
- biquad primitives, including RBJ coefficient helpers
- resampler primitives, added later
- metrics used by tests where appropriate

Scalar implementations are the source of truth for SIMD differential tests.
`gain`, `dcshift`, and linear `fade` have explicit backend-dispatched entry
points that keep scalar as the default and use the Auralis SIMD backend only
when requested.

### `auralis-effects`

Contains typed effect processors built from DSP primitives:

- `Gain`
- `Channels`
- `Norm`
- `DcShift`
- `Trim`
- `Pad`
- `Reverse`
- `Fade`
- `Vol`
- `Tremolo`
- `Overdrive`
- `Saturation`
- `Repeat`
- `Remix`
- `Centercut`
- `AllPass`
- `Band`
- `BandReject`
- `Bass`
- `Treble`
- `Equalizer`
- `HighPass`
- `LowPass`
- `Deemph`
- `Riaa`
- `Delay`
- `Downsample`
- `Upsample`
- `Speed`
- `Splice`
- `Stretch`
- `Synth`
- `Tempo`
- `Bend`
- `Echo`
- `Echos`
- `Flanger`
- `Phaser`
- `Reverb`
- `Biquad`
- `Oops`
- `Swap`
- `Compand`
- `MCompand`
- `NoiseProf`
- `NoiseRed`
- `Silence`
- `Vad`

Effect implementations should be whole-buffer-first and may keep local state
inside an op, but that state must not imply graph-level streaming, chunked, or
realtime execution.
The crate root is a small facade; effect-local behavior lives in focused
`gain`, `channels`, `norm`, `contrast`, `softvol`, `loudness`, `centercut`, `allpass`, `band`, `bandpass`, `bandreject`, `bass`, `treble`, `equalizer`, `highpass`, `lowpass`, `deemph`, `riaa`, `delay`, `downsample`, `upsample`, `speed`, `stretch`, `synth`, `tempo`, `pitch`, `bend`, `rate`, `chorus`, `compand`, `mcompand`, `noiseprof`, `noisered`, `stat`, `stats`, `flanger`, `phaser`, `reverb`, `echo`, `echos`, `oops`, `swap`, `tremolo`, `overdrive`, `saturation`, `silence`, `vad`, `repeat`, `remix`, `dcshift`, `trim`, `pad`, `reverse`, `fade`, and `vol` modules, with shared
typed errors in `error`.
The crate also owns the static effect registry and typed command parser used by
upcoming chain parsing. Implemented SoX-ng names such as `gain`, `dcshift`,
`trim`, `pad`, `repeat`, `remix`, `centercut`, `allpass`, `band`, `bandpass`, `bandreject`, `bass`, `treble`, `equalizer`, `highpass`, `lowpass`, `loudness`, `deemph`, `riaa`, `delay`, `downsample`, `upsample`, `speed`, `stretch`, `synth`, `tempo`, `pitch`, `bend`, `rate`, `chorus`, `compand`, `mcompand`, `noiseprof`, `noisered`, `stat`, `stats`, `flanger`, `phaser`, `reverb`, `echo`, `echos`, `oops`, `swap`, `reverse`, `fade`, `silence`, `vol`, `channels`, `norm`, `contrast`, `softvol`, `tremolo`, `overdrive`, and `saturation` resolve to typed descriptors; aliases such
as `dc-shift`, `eq`, `gain-db`, `volume`, `soft-volume`, and `normalize` resolve to their canonical names; unknown names
receive deterministic suggestions; and known SoX-ng effects without Auralis
coverage return a stable missing-coverage diagnostic. `dolbyb` is a stable
blocked exception rather than ordinary missing coverage: SoX-ng's available
path depends on GPLv2 `libdolbyb` C code, while Auralis is MIT and pure Rust, so
the CLI points users to `sox_ng ... dolbyb ...` or a compatible
pure-Rust/public-domain spec. `dop` is a stable not-planned effect-registry
exception: it is DSD-over-PCM transport packing from 1-bit DSD into 24-bit
samples, which belongs at a future DSD/DoP format boundary rather than in the
current PCM16 WAV effect pipeline. `ladspa` is a stable blocked exception:
SoX-ng-compatible LADSPA support requires loading native external plugins and
exposing a plugin-host ABI. `sdm` is a stable not-planned effect-registry
exception: it emits 1-bit DSD-style output through LGPL SoX-ng filter tables and
trellis behavior, which belongs behind a future DSD/1-bit format boundary
rather than in the current PCM16 WAV effect pipeline. Tokenized commands such
as `["gain", "-3"]`, `["trim", "48000", "48000"]`, and `["fade", "t",
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
through `gain -b`. The implemented `vol` command accepts amplitude, power, and
dB gain types, suffix forms such as `vol -6dB`, and SoX-ng limiter gain such as
`vol 2 amplitude 0.05`; unlike plain `gain`, `vol` clips inside the effect.
The implemented `norm` command accepts an optional dBFS level and performs
SoX-ng-style whole-buffer peak normalization at its chain position, separate
from Auralis' final output `--norm` policy. The implemented `contrast` command
accepts an optional amount in SoX-ng's `0..=100` range and defaults to `75`.
The implemented `softvol` command accepts `volume`, `double-time`, and
`headroom` arguments, lowers its current multiplier before any frame that would
clip, and optionally recovers upward according to the input sample rate.
The implemented `loudness` command accepts gain, reference-level, and FIR
half-length arguments in SoX-ng's supported ranges and applies scalar ISO 226
equal-loudness compensation.
The implemented `silence` command accepts `-l`, `above-periods`, sample-count
or seconds durations, percent or dB thresholds, and negative `below-periods`
for restart-based middle-silence removal.
The implemented `vad` command accepts SoX-ng's advanced timing, noise,
measurement, filter/lifter, trigger, search, gap, and pre-trigger options and
maps them onto Auralis' deterministic whole-buffer leading-trim detector.
The implemented `noiseprof` command accepts an optional profile output path
and exposes deterministic 2048-point FFT log-power profile generation for
later `noisered` use; chain execution preserves SoX-ng's audio pass-through
behavior.
The implemented `noisered` command accepts a profile path and optional
`0..=1` amount, loads SoX-ng-style `Channel N: ...` profile text, and applies
deterministic scalar overlap-window spectral reduction.
The implemented `stat` command accepts scale, RMS scaling, volume-only, and
JSON report options; chain execution passes audio through while the typed API
returns deterministic frame-major sample statistics.
The implemented `stats` command accepts signed-bit, hexadecimal-bit, floating
scale, window-time, and JSON report options; chain execution passes audio
through while the typed API returns deterministic overall and per-channel
sample statistics.
The implemented `synth` command accepts `-n`, optional seconds or frame-count
length, tonal waveforms `sine`, `square`, `sawtooth`, `triangle`, `trapezium`,
and `exp`, noise waveforms `whitenoise`, `tpdfnoise`, `pinknoise`, and
`brownnoise`, `:`, `+`, `/`, and `-` sweep forms, and `create`, `mix`, `amod`,
`fmod`, and `vdelay` combine modes.
The implemented `fir` command accepts SoX-ng-style coefficient input: no
arguments or `-` represent standard input, one argument is a coefficient-file
path even when it looks numeric, and two or more arguments are inline finite
coefficients. Chain execution supports inline coefficients and explicit
coefficient files, treats empty coefficient lists as a null effect, and applies
the scalar SoX-ng-aligned FIR impulse response.
The implemented `firfit` command accepts no arguments or `-` for stdin, one
knot-file path, or inline `<freq gain>` pairs with strictly increasing
non-negative frequencies and dB gains. Chain execution supports inline and
file-backed knots, emits exact centered-impulse FIR coefficients for flat
responses, and uses a deterministic scalar log-frequency interpolation
scaffold for non-flat response fitting.
The implemented `tremolo` command accepts a required speed in hertz and an
optional depth percentage, defaulting to SoX-ng's `40`.
The implemented `overdrive` command accepts optional `gain` and `color`
arguments in SoX-ng's `0..=100` range, applies cubic soft clipping plus the
channel-local stateful high-pass output blend, and preserves SoX-ng's
null-effect behavior for `overdrive 0`.
The implemented `saturation` command accepts `tanh`, `sqrt`, and `diode`
families with `blend`, `offset`, and family-specific `drive`, `color`, or
`threshold` parameters, then recenters and gain-compensates the wet path before
mixing it with the dry input.
The implemented `repeat` command accepts an optional finite count, defaults to
`1`, treats `0` as an identity transform, validates output length, and rejects
SoX-ng's unbounded `repeat -` form.
The implemented `downsample` command accepts an optional integer factor,
defaults to `2`, keeps the first frame of each factor-sized group, updates
sample-rate metadata to `input_rate / factor`, and performs no anti-alias
filtering.
The implemented `upsample` command accepts an optional integer factor, defaults
to `2`, inserts `factor - 1` zero frames after each input frame, updates
sample-rate metadata to `input_rate * factor`, and performs no reconstruction
filtering.
The implemented `speed` command accepts a required positive ratio or cents
argument with a `c` suffix, preserves decoded samples and frame count, updates
sample-rate metadata to `round(input_rate * factor)`, and leaves filtering or
resampling to later `rate` features.
The implemented `stretch` command accepts SoX-ng's basic cross-fade
`factor`, `window`, `fade`, `shift`, and `fading` arguments, preserves the
sample-rate metadata, and changes duration through channel-local scalar
window overlap processing.
The implemented `tempo` command accepts SoX-ng's `[-q] [-m|-s|-l] factor
[segment [search [overlap]]]` surface, preserves sample-rate metadata and
pitch, changes duration through scalar overlap-search processing, and uses
millisecond units for explicit segment/search/overlap tuning.
The implemented `pitch` command accepts SoX-ng's `[-q] shift [segment [search
[overlap]]]` surface, interprets shift in cents, preserves approximate
duration by reusing the inverse-factor tempo path, and updates sample-rate
metadata to `round(input_rate * 2^(shift / 1200))`.
The implemented `bend` command accepts SoX-ng's `[-f frame-rate] [-o oversample]
{start(+),cents,end(+)}` surface, preserves duration and sample-rate metadata,
and applies a channel-local scalar phase-vocoder path for linear pitch bends.
The implemented `rate` scaffold accepts a required target frequency such as
`rate 44100` or `rate 44.1k`, plus SoX-ng quality selectors `-q`, `-l`, `-m`,
`-g`, `-h`, `-e`, `-v`, `-u`, and equivalent `-Q 0` through `-Q 7` forms.
It also records SoX-ng control and override flags including `-i`, `-c`, `-f`,
`-n`, `-t`, phase options `-M`/`-I`/`-L`/`-p`, bandwidth options `-s`/`-b`/`-B`,
aliasing controls `-A`/`-a`, and precision options `-d`/`-R`. These options
currently share deterministic scalar linear resampling.
The implemented `remix` command accepts SoX-ng out-spec routing with
1-based channel numbers, ranges, open ranges, `-` for all channels, and
standalone `0` silent outputs. It supports `v` voltage, `p` power-dB, and `i`
inverted power-dB source modifiers, plus `-a` automatic scaling, `-m` manual
scaling, and `-p` power scaling. The default semi-automatic mode uses `1 / n`
scaling only when an output spec has no explicit gain modifiers.
The implemented `oops` command accepts no arguments and emits stereo
left-minus-right output in both channels; input must contain at least two
channels, and extra input channels are ignored.
The implemented `swap` command accepts no arguments and swaps adjacent decoded
channel pairs, leaving mono input and an odd trailing channel unchanged.
The implemented `reverb` command accepts `[-w] [reverberance [HF-damping
[room-scale [stereo-depth [pre-delay [wet-gain]]]]]]`, uses SoX-ng's
Freeverb-derived comb/all-pass delay network, preserves input length, and keeps
the command-line output channel shape stable.
The typed `Centercut` processor implements the core spectral center-cut
separation path and returns left residual, right residual, and extracted center
channels from stereo input. The implemented `centercut` command accepts
SoX-ng-style `-a gain`, `-b`, and `-w size` options for output gain,
bass-to-sides routing below 200 Hz, and power-of-two spectral window sizes.
The implemented `allpass` command accepts `allpass frequency width` plus
`allpass -1 frequency` and `allpass -2 frequency`, supports hertz, kilohertz,
Q, and octave width suffixes, and rejects frequencies at or above Nyquist for
the input sample rate.
The implemented `band` command accepts `band [-n] frequency [width]`, uses
SoX-ng's historical resonator formula, defaults width to `frequency / 2`, and
supports hertz, kilohertz, Q, and octave width suffixes.
The implemented `bandpass` command accepts `bandpass [-c] frequency width`,
uses the RBJ band-pass helpers, and supports hertz, kilohertz, Q, and octave
width suffixes.
The implemented `bandreject` command accepts `bandreject frequency width`, uses
the RBJ notch helper, and supports hertz, kilohertz, Q, and octave width
suffixes.
The implemented `bass` and `treble` commands accept
`gain [frequency [width]]`, use RBJ low-shelf and high-shelf helpers
respectively, default to SoX-ng's 100 Hz and 3000 Hz shelf midpoints, and
support shelf slope plus hertz, kilohertz, Q, and octave width suffixes.
The implemented `equalizer` command accepts `frequency width gain`, uses the
RBJ peaking-EQ helper, and supports hertz, kilohertz, Q, and octave width
suffixes.
The implemented `highpass` command accepts `highpass [-1|-2] frequency
[width]`, defaults omitted width to `0.707q`, uses the RBJ high-pass helper for
two-pole forms, and supports hertz, kilohertz, Q, and octave width suffixes.
Parsed `EffectCommand` values render back to canonical SoX-ng-style token
vectors using stable effect names, explicit default arguments, and deterministic
numeric formatting, so equivalent values such as `gain`, `gain 0`, and
`gain-db 0.0` produce the same manifest representation. `EffectChain` groups
typed commands into an in-memory sequential chain, preserves explicit `:`
boundaries between chain segments, applies implemented commands in caller
order, supports forced scalar/SIMD backend selection for backend-aware effects,
and reports processing failures with the zero-based command index, canonical
command tokens, failed argument family, and typed source error. Flat token
streams can also be parsed into an `EffectChain`, which is how `auralis render
<input> -o <output> --fx "gain -3 : reverse"` shares the same ordering and
diagnostics as the library API. The `newfile` and `restart` controls are reserved for later
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
with `auralis render input.wav -o output.wav --effects-file chain.effects`;
effects files are mutually exclusive with `--fx` and `--chain` because their
relative order would otherwise be ambiguous. Empty boundary
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
auralis convert input.wav -o output.wav
auralis render input.wav -o output.wav
auralis render input.wav -o output.wav --fx 'gain -3'
auralis render input.wav -o output.wav --backend simd --fx 'gain -3'
auralis render input.wav -o output.wav --fx 'dcshift 0.125'
auralis render input.wav -o output.wav --backend simd --fx 'dcshift 0.125'
auralis render input.wav -o output.wav --fx 'trim 48000 =96000'
auralis render input.wav -o output.wav --fx 'trim 1 =2'
auralis render input.wav -o output.wav --fx 'trim 48000 24000 -12000'
auralis render input.wav -o output.wav --fx 'pad 24000 48000'
auralis render input.wav -o output.wav --fx 'pad 24000@12000'
auralis render input.wav -o output.wav --fx 'fade t 24000'
auralis render input.wav -o output.wav --backend simd --fx 'fade t 24000 0 24000'
auralis render input.wav -o output.wav --fx reverse
auralis render input.wav -o output.wav --fx 'gain -3' --fx 'dcshift 0.125' --fx reverse
auralis render input.wav -o output.wav --fx 'gain -3 : dcshift 0.125 reverse'
auralis render input.wav -o output.wav --backend simd --fx 'gain -3 fade t 24000 0 24000'
auralis render input.wav -o output.wav --effects-file chain.effects
auralis render first.wav -o output.wav --combine concatenate --input second.wav --fx reverse
auralis render first.wav -o output.wav --combine sequence --input second.wav --fx reverse
auralis render first.wav -o output.wav --combine mix --input second.wav --fx reverse
auralis render first.wav -o output.wav --combine mix-power --input second.wav --fx reverse
auralis render first.wav -o output.wav --combine merge --input second.wav --fx reverse
auralis render first.wav -o output.wav --combine multiply --input second.wav --fx reverse
auralis convert stereo.wav -o mono.wav --channels 1
auralis run Auralis.toml
auralis completions zsh
```

Typed effect commands are passed through `--fx` or `--chain` after the input
and output paths. A `:` token still preserves an explicit chain boundary in the
parsed representation and deterministic rendering; current processing still
executes the implemented commands sequentially in memory. SoX-ng `newfile` and
`restart` boundary controls are recognized but rejected until their own
pipeline features exist. Backend selection remains an option, and
`--effects-file` is mutually exclusive with `--fx` and `--chain` because their
relative order would otherwise be ambiguous. Effects files use the same parser
as the library `parse_effects_file` API.

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
with `--norm`.

Output dither insertion follows the same explicit output-boundary model. The
high-level library defaults to `OutputDitherPolicy::Disabled`, so current copy
and test paths never get hidden noise. Callers can use
`Pipeline::with_output_dither` or
`Pipeline::with_output_dither_policy(OutputDitherPolicy::Automatic(_))` to apply
deterministic TPDF dither after rate, channel, guard, and norm policies and
before PCM16 encoding. The CLI exposes this as `--dither`; `--dither-seed`
selects a repeatable PRNG seed and is rejected unless `--dither` is present.
SoX-ng comparison manifests use `sox_ng_auto_dither = true` when their reference
command intentionally runs without `-D` and lets SoX-ng auto-insert dither.

Selected crates:

- `clap` for the parser.
- `clap_complete` for shell completions.
- `clap_mangen` for generated man pages.

### `auralis-testkit`

Contains test utilities shared by Rust tests and auxiliary Python runners:

- shared deterministic corpus generation by stable ID for silence, impulse,
  step, sine, sweep, seeded noise, full-scale, near-zero, odd-length,
  mono/stereo, and short-buffer cases
- raw f32 helpers
- WAV decode helpers
- metric calculation for max absolute error, RMS error, SNR, peak, and DC offset
- golden test manifest handling for standalone effects, chain cases, combine
  modes, output policies, and SoX-ng automatic behavior metadata
- scalar-vs-SIMD backend conformance helpers
- SoX-ng command wrapper
- tolerance definitions
- shared `auralis.golden.failure.v1` failure artifact generation for Rust and
  Python golden runners

Rust/testkit is the primary verification surface. Python is kept as an
auxiliary runner for cross-tool execution, numerical helpers, compatibility
checks, and report generation; it must not be the only critical behavior
verification path. The complete testing policy is maintained in
[testing.md](testing.md).

Golden manifests use TOML tables keyed under `id`:

```toml
[id.gain_minus_3_mono]
input = "sine_48k_mono.wav"
corpus_id = "l0/sine_mono_32"
auralis = ["gain", "-3"]
sox_ng = ["gain", "-3"]
max_abs = 1e-4
rms = 1e-6
snr_db = 90.0
```

The `auralis` array is appended after `auralis render <input> -o <output>`, while
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

Manifest coverage is intentionally summarized here rather than repeated in
effect-by-effect detail. Standalone effect cases, chain cases, combiner cases,
auto-channel/rate/level/dither policies, failure artifact contents, and future
complex pipeline golden requirements are described in [testing.md](testing.md)
and the checked-in manifests under `tests/golden/`.

### `auralis-python` future placeholder

Python support should be planned but not implemented until the Rust API is
stable. Feature 9.1 is blocked until the Rust API, effect pipeline behavior,
error model, buffer model, and binding documentation preconditions are
re-audited and pass.

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

Users of `auralis-core` should not see names such as `symphonia`, `hound`,
`clap`, `rten-simd`, `rubato`, `pyo3`, or `ndarray` in public API types.

### Add in the first workspace pass

| Area | Choice | Boundary |
|---|---|---|
| CLI | `clap`, `clap_complete`, `clap_mangen` | `auralis-cli`; generated help/completions/man pages from one definition |
| Errors | `thiserror`, `miette`, small amounts of `anyhow` | `thiserror` for typed library errors; `miette` and `anyhow` stay in binaries, tests, and glue |
| Config and reports | `serde`, `toml`, `serde_json` | pipeline manifests are TOML; machine-readable reports are JSON |
| Codec decode | `symphonia` | wrapped inside `auralis-codec`; never exposed by `auralis-core` |
| WAV fallback | `hound` | used only when Symphonia cannot cover an Auralis-supported WAV case |
| Observability | `tracing`, `tracing-subscriber` | libraries emit structured events; CLI initializes subscribers |
| Float assertions | `approx` | dev/test only |
| Property tests | `proptest` | dev/test only |
| CLI tests | `assert_cmd`, `predicates`, `tempfile` | dev/test only |
| Snapshots | `insta` with `serde` | dev/test only; for help, diagnostics, manifests, and reports |
| Benchmarks | `criterion` | dev/bench only |

Library APIs must return typed errors, not `anyhow::Result<T>`. `miette` is for
diagnostic presentation at the CLI boundary. Symphonia handles normal decode
inside `auralis-codec`; `hound` handles only the documented WAV fallback cases.
Auralis tests compare decoded PCM and metadata rather than whole file bytes
unless a test is specifically about serialization.

Future decode support goes through `auralis-codec` unless DEVELOPMENT
explicitly changes that policy. Codec adapters may depend on audited backend
crates behind feature gates, but the current roadmap does not plan external
`ffmpeg` command backends, `ffmpeg-next`, libFLAC wrappers, LAME wrappers,
libvorbis wrappers, `libopusenc`, FDK-AAC, other native codec-library bindings,
new per-format target crates for complex codecs, or non-WAV encode paths before
an explicit encoder policy exists. FLAC encode through `flacenc` is the named
planned exception.

### Selected direction, but optional or later

| Area | Choice | Rule |
|---|---|---|
| SIMD | `rten-simd` behind `simd` | optional backend only; scalar remains the reference |
| Frequency-domain effects and tests | `rustfft`, later `realfft` if needed | `rustfft` is used inside `auralis-effects` for `bend`; do not expose FFT crates through public core APIs |
| FLAC encode | `flacenc` | planned behind `auralis-codec`; Symphonia remains decode-only |
| Batch parallelism | `rayon` behind `parallel` | for many files, test cases, stems, or render jobs; not the initial single-stream effect chain |
| Byte casting | `bytemuck` behind `pod` | only after normal parsing is correct and profiling justifies it |
| Small allocation optimization | `smallvec` behind `smallvec` | only for proven small-vector pressure |
| Python package | `pyo3`, `maturin`, `numpy` | future `auralis-py`; keep Rust API and buffer model ready |

### Do not introduce now

| Crate | Decision |
|---|---|
| `rubato` | Do not make it the core resampler. Later it may be a reference or benchmark target against Auralis scalar rate and SoX-ng golden tests. |
| `ffmpeg` / `ffmpeg-next` | Not planned under the current codec facade policy. |
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
symphonia = { version = "0.5", default-features = false, features = ["pcm", "wav"] }
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
