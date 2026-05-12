# Auralis Status

## Project status

Auralis is currently pre-alpha. The repository contains the initial Rust
workspace skeleton, uv-based Python test harness, and core audio type
vocabulary with planar internal audio buffers. The codec trait boundary is in
place for WAV-first scope and explicit unsupported-format reporting. PCM8 and
PCM16 WAV decoding into planar `f32` buffers are implemented, PCM16 legacy WAV
writing remains in place, PCM8/PCM16 writing is available through the newer
output-format boundary, the `auralis inspect` CLI reports PCM16 WAV metadata, and
`auralis run input.wav output.wav` performs a decode-through-buffer copy
pipeline and can apply constant gain with `--gain-db <DB>`, SoX-ng-style
multi-range trim positions, zero padding with frame counts and insertion
positions, frame-level
reversal with `--reverse`, constant DC offset with `--dc-shift <SHIFT>`, or
linear fades with `--fade-in-frame <FRAMES>` and `--fade-out-frame <FRAMES>`.
The scalar `gain`, `dcshift`, `fade`, and biquad DSP primitives, the typed `Gain`, `Channels`, `Norm`,
`Contrast`, `SoftVol`, `Centercut`, `AllPass`, `Band`, `BandPass`, `BandReject`, `Bass`, `Treble`, `Equalizer`, `HighPass`, `Hilbert`, `Sinc`, `Dither`, `LowPass`, `Deemph`, `Riaa`, `Delay`, `Downsample`, `Upsample`, `Speed`, `Splice`, `Stretch`, `Tempo`, `Pitch`, `Rate`, `Echo`, `Echos`, `Chorus`, `Flanger`, `Phaser`, `Reverb`, `Biquad`, `Oops`, `Swap`, `Tremolo`, `Overdrive`, `Saturation`, `Repeat`, `Remix`, `DcShift`, `Trim`, `Pad`, `Reverse`, `Fade`,
`Compand`, `MCompand`, `NoiseProf`, `NoiseRed`, `Stat`, `Stats`, `Synth`, `Fir`, `FirFit`, `Silence`, `Vad`, and `Vol` effect processors, the high-level library chain API for applying
gain, channels, norm, contrast, softvol, loudness, centercut, allpass, band, bandpass, bandreject, bass, treble, equalizer, highpass, hilbert, sinc, dither, lowpass, deemph, riaa, delay, downsample, upsample, speed, splice, stretch, synth, tempo, pitch, rate, chorus, compand, mcompand, flanger, phaser, reverb, echo, echos, biquad, oops, swap, tremolo, overdrive, saturation, repeat, remix, dcshift, trim, pad, reverse,
fade, fir, firfit, noiseprof, noisered, stat, stats, silence, vad, and vol, and the CLI gain/channels/norm/contrast/softvol/loudness/centercut/allpass/band/bandpass/bandreject/bass/treble/equalizer/highpass/hilbert/sinc/dither/lowpass/deemph/riaa/delay/downsample/upsample/speed/splice/stretch/synth/tempo/pitch/rate/chorus/compand/mcompand/fir/firfit/noiseprof/noisered/stat/stats/flanger/phaser/reverb/echo/echos/biquad/oops/swap/tremolo/overdrive/saturation/repeat/remix/dcshift/trim/pad/reverse/fade/silence/vad/vol transforms are implemented. The Rust
effects crate also exposes a deterministic name registry and typed command
parser for the implemented effect subset; supported names and aliases resolve
to typed descriptors, parsed command tokens become typed effect configs, and
unknown names or unsupported SoX-ng options return stable diagnostics. Parsed
commands can be grouped into an in-memory `EffectChain` and executed in order
with indexed command-context errors and forced scalar/SIMD backend selection.
The chain path supports executable SoX-ng-style `compand` commands with shared
or per-channel envelope followers, the reusable companding transfer-function
model, optional post gain, initial volume, and length-preserving look-ahead
delay. It also supports `mcompand` quoted band groups with ascending crossover
frequencies; nonzero mcompand band delays are rejected because SoX-ng's own
mcompand flow path treats them as unsupported at runtime.
It also supports `noiseprof [profile-file(-)]` as a pass-through analyzer with
typed profile generation and SoX-ng-style channel-major profile text rendering.
It supports `noisered [profile-file(-) [amount]]` as a scalar FFT-domain noise
reducer that consumes the same profile text and mirrors SoX-ng's overlapping
window output shape.
It supports `stat [-s scale] [-rms] [-v] [-j]` as a pass-through analyzer with
a typed deterministic report API for SoX-ng-style sample statistics.
It supports `stats [-b bits|-x bits|-s scale] [-w window-time] [-j]` as a
pass-through analyzer with typed deterministic overall and per-channel sample
statistics.
It supports `synth [-n] [length] waveform [combine] [frequency-sweep]` as a
deterministic SoX-ng-style waveform and noise generator over the decoded input
shape, including sweep and input-combine modes.
The chain path supports SoX-ng-style `gain -h` and `gain -r` headroom metadata,
`gain -n` peak normalization, `gain -l` limiting, and channel-aware `gain -e`,
`gain -B`, and `gain -b` scans: `gain -h DB` applies the fixed attenuation and
records reclaimable headroom, and a later `gain -r` restores as much as
possible without clipping. The chain path also supports SoX-ng-style `vol`
amplitude, power, dB, and limiter-gain forms, effect-level `norm [level]` as a
positioned normalization command distinct from final output `--norm`, and
SoX-ng-style `contrast [amount]` phase contrast enhancement, `softvol`
clipping-avoidant volume control with optional recovery and headroom,
`loudness [gain [reference [n]]]` ISO 226 loudness compensation, and
`tremolo speed [depth]` sinusoidal amplitude modulation, and
`overdrive [gain [color]]` cubic soft-clipping distortion, and
`saturation [type [blend [offset [drive|color|threshold]]]]` nonlinear
saturation, `silence [-l] above-periods ...` leading/trailing/middle silence trimming, `vad` leading non-voice trimming with SoX-ng-style advanced option parsing, finite `repeat [count]` output duplication, `oops` out-of-phase stereo extraction, `swap` adjacent channel-pair exchange, and
`remix [-a|-m] [-p] out-spec...` channel routing with source gain modifiers.
The chain path also supports SoX-ng-style `reverb [-w]` with reverberance,
HF damping, room scale, stereo depth, pre-delay, and wet-gain parameters. It is
length-preserving, preserves the command-line output channel shape, and
intentionally does not drain the delayed wet tail after the input.
The chain path also supports explicit SoX-ng-style `channels number` conversion
at a user-visible effect position, using the same conversion primitive as the
output `--channels` policy, and `rate [quality/options] frequency` conversion
with a deterministic scalar linear scaffold for all implemented SoX-ng quality
and override metadata. `auralis run <input.wav> <output.wav> gain -3 channels 1 rate -h -M -s -R 120 44100 norm -6 contrast softvol 2 loudness -6 65 127 allpass 1000 0.707q band -n 1000 2q bandpass -c 1000 2q bandreject 1000 2q bass 6 treble -6 equalizer 1000 1q 6 highpass 500 lowpass 1000 riaa chorus -l 0.5 1 1 0.25 1 0 flanger -l 0 0 0 100 1 phaser -l 0.4 0.74 3 0.4 0.5 reverb 50 50 100 0 0 0 echo 0.5 1 1 0.5 echos 0.5 1 1 0.25 biquad 0.5 0 0 1 -0.5 0 tempo 1.25 pitch 1200 tremolo 5 overdrive 12 25 saturation sqrt 0.75 0.1 0.25 silence 0 repeat 1 remix 1 oops swap dcshift 0.125 reverse` exposes the same typed chain model at the CLI,
preserving positional user order while the earlier single-effect flags remain
available for compatibility. The golden
suite now includes standalone effect coverage in `tests/golden/effects.toml`
plus a `tests/golden/chains.toml` manifest for representative editing, level,
gain headroom/reclaim, and fade/gain filter-style positional chains against
SoX-ng.
The biquad support now exposes both the reusable scalar primitive and the
SoX-ng-style `biquad b0 b1 b2 a0 a1 a2` command. Raw command coefficients are
normalized by `a0`, invalid coefficient sets are rejected before processing,
and filtering preserves independent per-channel state. The primitive also
provides RBJ coefficient helpers and SoX-ng-compatible width units for future
shelf and EQ-style filter effects. The implemented `allpass` command
uses those helpers for `allpass frequency width` and also supports SoX-ng's
`allpass -1 frequency` and `allpass -2 frequency` alternate all-pass forms.
The implemented `band` command covers SoX-ng's resonator band-pass filter,
including the default `frequency / 2` width and `-n` unpitched/noise scaling.
The implemented `bandpass` command covers SoX-ng's RBJ band-pass filter with
constant 0 dB peak gain by default and constant-skirt `-c` scaling when
requested.
The implemented `bandreject` command covers SoX-ng's RBJ notch filter shape
with the same hertz, kilohertz, Q, and octave width units.
The implemented `lowpass` command covers SoX-ng's default RBJ two-pole low-pass
filter with optional width units and the single-pole `-1` form.
The implemented `highpass` command covers SoX-ng's default RBJ two-pole
high-pass filter with optional width units and the single-pole `-1` form.
The implemented `hilbert` command covers SoX-ng's Blackman-windowed Hilbert
transform FIR filter with default sample-rate-derived taps and explicit
odd tap-count overrides.
The implemented `sinc` command covers SoX-ng-style low-pass, high-pass,
band-pass, and band-reject Kaiser-windowed FIR filters with attenuation, beta,
transition-bandwidth, explicit-tap, auto-tap rounding, and low-pass
delete-at-Nyquist options.
The implemented `dither` command covers deterministic plain TPDF, `-S` sloped
TPDF, and Shibata noise shaping through `-s` or `-f shibata`, with `-p bits`
target precision and explicit typed seed configuration. Automatic on/off
detection (`-a`) and additional named shaping filters remain later roadmap
items.
The implemented `loudness` command covers SoX-ng's ISO 226 equal-loudness
FIR compensation with gain, reference-level, and half-length arguments.
The implemented `riaa` command covers SoX-ng's no-argument RIAA playback
equalization filter at 44.1 kHz, 48 kHz, 88.2 kHz, 96 kHz, and 192 kHz with
0 dB normalization at 1 kHz.
The implemented `chorus` command covers SoX-ng's `-n`, `-l`, and `-q`
interpolation flags, `-s` and `-t` default wave selection, per-stage
`-sine`/`-triangle` wave overrides, multiple delay stages, clipping, and
explicit delay-line tail extension.
The implemented `flanger` command covers SoX-ng's `-n`, `-l`, and `-q`
interpolation flags, `-s` and `-t` wave selection, delay/depth/regen/width,
speed, shape, and phase arguments, feedback, balanced dry/wet mixing, clipping,
and length-preserving output without a delayed tail.
The implemented `phaser` command covers SoX-ng's `-n`, `-l`, and `-q`
interpolation flags, `-s` and `-t` wave selection, gain-in/gain-out, delay,
regen, and speed arguments, channel-local feedback delay lines, clipping, and
length-preserving output without a delayed tail.
The Rust testkit owns deterministic corpus generation, TOML golden manifests,
SoX-ng command rendering, comparison metrics, failure reports, and
scalar-vs-SIMD conformance helpers. Python pytest remains an auxiliary
cross-tool/reporting layer; the full test policy lives in
[testing.md](testing.md).
The effects crate also parses
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
L4 property/metamorphic and L5 chunk-invariance coverage are tracked through
the Rust testkit and the layered coverage matrix described in
[testing.md](testing.md).
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
The 7.x DSP primitive audit in `doc/development/07-dsp-primitives.md` records
shared-kernel ownership across biquad, FIR, modulation, delay-line, time-scale,
spectral, dither, resampling, mixing, and analyzer primitives. The first
extractions moved the direct-form biquad runtime, normalized coefficient type,
generic RBJ helper ownership, reusable FIR coefficient validation, and centered
FIR state, and deterministic dither/noise state plus quantization helpers into
`auralis-dsp` while preserving `auralis-effects` command parsing and public
compatibility wrappers. Dither SIMD remains intentionally not applicable
because the PRNG and noise-shaping state order is semantically observable.
The 8.0 format-support policy now records that future codec backends must stay
pure Rust unless the development plan changes explicitly. WAV remains the
built-in adapter path, RAW PCM is planned as an Auralis-owned boundary,
AIFF/AIFC is classified as feature-gated pure Rust adapter work, FLAC is
classified as experimental pure Rust adapter work, and external `ffmpeg` or
native codec wrappers remain not planned under the current roadmap. The codec
boundary now also owns `OutputFormat`, per-format encode option models,
`EncodeSummary`, and the `AudioEncoder` trait; the current WAV adapter plugs
into that surface while future RAW PCM, AIFF/AIFC, and FLAC writes return
typed unsupported-format errors until their individual format leaves land.
Feature 8.1.1 is now complete: WAV PCM8 decode support has joined the generic
high-level WAV open path, and WAV writes can now target PCM8 or PCM16 through
`WavEncodeOptions` while the legacy `write_wav` path stays PCM16-only for
backward compatibility.
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
