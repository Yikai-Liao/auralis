---
kind: historical-roadmap
status: superseded
superseded_by:
  - dsp.md
  - simd.md
---

# 7.x Reusable DSP Primitives Roadmap

This is the historical roadmap for extracting reusable DSP kernels after the
specialized-effect classification work. It is retained for analysis history and
coverage evidence; new DSP primitive planning belongs in `dsp.md`, `dsp/*.md`,
`simd.md`, and `simd/*.md`.

Historical entries below may mention streaming or chunked state contracts.
Those notes are not active graph-execution goals; ADR 0004 keeps Auralis graph
execution whole-buffer only.

## Scope

Many DSP kernels currently live inside `auralis-effects` effect-local modules.
That is acceptable for one-off behavior, but it creates pressure to duplicate
shared algorithms as more effects are added. The next stage must decide, case by
case, whether a kernel should move into `auralis-dsp` or remain effect-local.

Do not split every effect into a DSP crate. Lower only kernels that satisfy at
least one of these conditions:

- reused by more than one effect or planned effect family;
- needs backend dispatch or scalar-vs-SIMD conformance in isolation;
- needs analytical tests that are clearer outside an effect command surface;
- forms a stable internal primitive without exposing implementation crate types
  through public `auralis-core` APIs.

Effect-specific glue, SoX-ng command parsing, diagnostics, registry entries,
and compatibility quirks should stay in `auralis-effects` unless they become
shared primitives themselves.

## Candidate Inventory

The audit must classify at least these primitive families:

- biquad state, coefficient normalization, and filter design helpers;
- FIR convolution, coefficient parsing/design helpers, and centered alignment;
- envelope followers, attack/decay smoothing, and transfer curves;
- modulation oscillators, wave-shape generation, and phase handling;
- delay lines, interpolation, feedback, and explicit tail/flush state;
- windows, overlap-add/search helpers, and crossfade curves;
- FFT/STFT helpers, spectral profiles, and overlap state;
- deterministic noise, dither, noise shaping, and quantization helpers;
- resampling scaffolds, rate metadata transforms, and future polyphase cores;
- mixing, channel mapping, gain staging, clipping, and full-scale policies;
- analyzer metrics, RMS/peak windows, and report accumulation state.

## Feature 7.1 Audit Result

The first audit pass is intentionally conservative: a primitive should move
only when extraction gives the next implementation a clearer owner, better
isolated tests, or a future SIMD/backend seam. Command parsing, SoX-ng
diagnostics, registry entries, and compatibility quirks remain in
`auralis-effects`.

| Primitive family | Current owner | Known users and likely users | Decision | Scalar/SIMD status | Required tests before extraction |
|---|---|---|---|---|---|
| Direct-form biquad runtime state and normalized coefficients | `crates/auralis-dsp/src/biquad.rs`, re-exported by `auralis-effects` | Current users: `biquad`, `allpass`, `band`, `bandpass`, `bandreject`, `bass`, `treble`, `equalizer`, `lowpass`, `highpass`, `deemph`, `riaa`, chain chunk-state tests. Likely users: more tone and filter effects. | Moved to `auralis-dsp` in Feature 7.2. `auralis-effects` keeps command parsing, diagnostics, registry entries, effect-specific presets, and public re-exports. | Scalar stateful reference exists. SIMD stays N/A because recursive IIR state is not a simple data-parallel kernel. | Preserve analytical coefficient/runtime tests, effect regressions for every biquad-backed effect, L5 chunk invariance, SoX-ng goldens for concrete effects, and doc tests for re-exports. |
| RBJ width units and cookbook coefficient helpers | `crates/auralis-dsp/src/biquad_design.rs`, re-exported by `auralis-effects` | Current users: tone/filter effects listed above. Likely users: future EQ/filter families. | Moved with the biquad coefficient type in Feature 7.2 because inherent RBJ helper methods must live in the crate that owns `BiquadCoefficients`. Width parsing remains in command modules. | Scalar coefficient math only. SIMD not applicable. | Keep closed-form coefficient tests and invalid-design coverage; verify concrete effect command rendering and SoX-ng golden behavior do not change. |
| FIR coefficient storage, centered convolution state, and tail flush | `crates/auralis-dsp/src/fir.rs`, wrapped by `auralis-effects` command/source handling | Current users: `fir`, `sinc`, `hilbert`, `loudness`, `earwax` uses a separate fixed FIR state. Likely users: more FIR filter families and future resampling filters. | Moved to `auralis-dsp` in Feature 7.3. `auralis-effects` keeps coefficient text/file/stdin semantics, diagnostics, command parsing, registry entries, typed effect wrappers, and public compatibility wrappers. | Scalar stateful reference exists for shared FIR. SIMD remains N/A until a vectorized convolution backend is deliberately added. | Preserve FIR analytical alignment tests, sinc/hilbert/loudness/effect regressions, chunk-state tests, SoX-ng goldens, and explicit tail/finish behavior tests. |
| Window design helpers for FIR filters | `sinc.rs`, `hilbert.rs`, `loudness.rs`, and `noisered.rs` | Current users: Kaiser windowed `sinc`, Blackman-windowed `hilbert`, equal-loudness FIR, Hann-windowed noise reduction. Likely users: future FIR/filter design features. | Keep effect-local until at least two concrete effects need the same parameterized helper. The current formulas are tightly coupled to SoX-ng command semantics and accepted ranges. | Scalar coefficient math only. SIMD not applicable. | Extraction would need analytical tests for window coefficients and unchanged SoX-ng goldens for every affected effect. |
| Envelope followers, attack/decay smoothing, and dB transfer curves | `compand.rs` and `mcompand.rs` | Current users: `compand`, `mcompand`. Likely users: compressor/expander or limiter effects. | Deferred by Feature 7.4. The transfer curve and follower are reusable, but current behavior is heavily SoX-ng-shaped, `mcompand` reuses whole `Compand` command configs, and delay/look-ahead state is intertwined with planar/interleaved conversion. Extract only after a narrower streaming-state contract exists. | Scalar stateful reference exists. SIMD N/A until a backend can process independent channels or bands without changing envelope ordering. | Preserve transfer-curve analytical tests, shared/per-channel envelope tests, mcompand crossover tests, L5 chunk-invariance if a streaming state is exposed, and SoX-ng goldens. |
| Modulation oscillators, waveform generation, and phase handling | `chorus.rs`, `flanger.rs`, `phaser.rs`, `tremolo.rs`, `synth.rs` | Current users: chorus/flanger/phaser LFOs, tremolo envelope, synth waveform/noise generation. Likely users: vibrato, more modulation effects. | Audit again after biquad/FIR. There is clear duplication pressure, but each effect currently uses slightly different phase, range, and interpolation semantics. Start by extracting only a tiny waveform helper if a future feature needs it. | Scalar math exists in effect-local code. SIMD mostly N/A for stateful LFO setup; generated sample loops might later get differential tests. | Need waveform analytical tests, phase-wrap tests, effect regressions, SoX-ng goldens, and fuzz seeds for waveform option parsing where command surfaces are touched. |
| Delay lines, interpolation, feedback, and explicit flush state | `delay.rs`, `echo.rs`, `echos.rs`, `chorus.rs`, `flanger.rs`, `phaser.rs`, `synth.rs` | Current users: fixed delay, echo taps, chorus/flanger/phaser feedback buffers, synth variable delay. Likely users: vibrato, reverb/all-pass comb helpers. | Candidate after modulation audit. Keep whole-buffer `delay` effect local for now; a reusable primitive should be a streaming/channel-local delay line with explicit interpolation and flush contracts. | Scalar stateful code exists in several effect-local forms. SIMD N/A until the API separates per-sample state from channel-parallel dispatch. | Need analytical delay-line tests, interpolation tests, tail/flush tests, effect-level regressions, and SoX-ng goldens for each affected effect. |
| Windows, overlap-add/search, crossfade curves, and time-scale state | `stretch.rs`, `tempo.rs`, `pitch.rs`, `splice.rs`, `bend.rs` | Current users: time-domain stretch/tempo/pitch, splice crossfades, bend segment resampling. Likely users: higher quality time-scale and pitch work. | Keep effect-local until the tempo/stretch implementations stabilize further. These routines are algorithm-specific and extraction would be high-risk without a smaller shared kernel. | Scalar whole-buffer reference exists. SIMD N/A for current search/overlap state. | Need deterministic window/crossfade analytical tests, whole-buffer shape tests, chunk-state design before L5 can apply, and SoX-ng goldens. |
| FFT/STFT spectral profiles and overlap state | `noiseprof.rs` and `noisered.rs` | Current users: noise profile collection and noise reduction. Likely users: spectral analysis, spectral gating, future STFT effects. | Blocked from immediate extraction. The current code depends on `rustfft` inside `auralis-effects`; moving it would require deciding whether `auralis-dsp` owns FFT dependencies and spectral profile formats. | Scalar FFT-domain reference through `rustfft`. SIMD/backend dispatch not currently controlled by Auralis. | First record dependency ownership, profile serialization boundaries, golden coverage, and regression tests for profile text compatibility. |
| Deterministic noise, dither, noise shaping, and quantization | `crates/auralis-dsp/src/dither.rs`, effect/output wrappers in `crates/auralis-effects/src/dither.rs`, and synth-local noise in `synth.rs` | Current users: `dither`, explicit output-boundary dither policy, synth noise waveforms. Likely users: automatic output dither, future noise generators, quantizers, and format boundaries. | Feature 7.5 moved the deterministic dither configuration, PRNG/noise-shaped state, and quantization helper into `auralis-dsp`; SoX-ng command parsing, CLI/output-policy choices, diagnostics, and compatibility wrappers remain in boundary crates. Synth waveform and noise generation deliberately remain effect-local. | Scalar stateful reference exists. SIMD N/A for the stateful PRNG/noise-shaping loop because sample order and seed history are observable behavior. | Preserve deterministic seed tests, quantization bounds, noise-shape regression, output-policy integration tests, chunk invariance, explicit dither SoX-ng goldens, and automatic output-dither goldens. |
| Resampling scaffolds, interpolation, and rate metadata transforms | `rate.rs`, `speed.rs`, `pitch.rs`, `bend.rs`, `downsample.rs`, `upsample.rs` | Current users: scalar linear `rate`, metadata speed, pitch/tempo composition, bend segment resampling, simple up/downsample effects. Likely users: future polyphase resampler. | Keep current scaffolds effect-local until the format/effect roadmap asks for a real resampling primitive. A future polyphase core should start in `auralis-dsp`, but the current linear scaffold is not the desired long-term API. | Scalar whole-buffer reference exists; future polyphase may need scalar-vs-SIMD tests. | Need analytical interpolation tests, sample-rate metadata tests, output-policy/effect equivalence tests, SoX-ng goldens where comparable, and scalar-vs-SIMD differential tests for a future kernel. |
| Mixing, channel mapping, gain staging, clipping, and full-scale policies | `remix.rs`, `channels.rs`, `gain.rs`, `norm.rs`, `vol.rs`, `softvol.rs`, chain/output-policy modules, existing kernels in `auralis-dsp`/`auralis-simd` | Current users: channel conversion, remix, combiners, gain/headroom, output guard/norm/dither policies. Likely users: additional mix/combine and format-boundary work. | Partially extracted already for simple gain/dc/fade/mix/multiply kernels. Keep policy-heavy pieces in `auralis-effects` or pipeline modules; only leaf sample kernels should move to `auralis-dsp`/`auralis-simd`. | Scalar and SIMD exist for simple data-parallel kernels. Policy scans and routing are scalar/structural. | Preserve scalar-vs-SIMD differential tests, effect/CLI equivalence, output-policy goldens, and property tests for clipping/finite output. |
| Analyzer metrics, RMS/peak windows, and report accumulation state | `stat.rs`, `stats.rs`, `noiseprof.rs` | Current users: `stat`, `stats`, noise profile collection. Likely users: meters, loudness reports, coverage metrics. | Keep effect-local until a second non-SoX report surface needs the same accumulator. Report formats and field names are command-facing and should not leak into `auralis-dsp`. | Scalar whole-buffer analyzers. SIMD N/A unless a future scan kernel is backend-dispatched. | Need report-field analytical tests, JSON/text rendering regressions, non-finite sample diagnostics, and SoX-ng parity tests where comparable. |

### Extraction Order From This Audit

1. Completed: move the direct-form biquad runtime, normalized coefficient type,
   `BiquadWidth`, and generic RBJ coefficient helpers into `auralis-dsp`, with
   `auralis-effects` re-exporting the public names and preserving command
   parsing.
2. Completed: split the FIR numeric primitive from command-style coefficient
   sources, then move the reusable validated coefficient container and centered
   `FirState` into `auralis-dsp`.
3. Completed: extracted deterministic dither/noise state into `auralis-dsp`.
   It has two current call surfaces (`dither` and output-boundary dither), a
   small self-contained state type, and existing seed/chunk/golden coverage.
   `auralis-effects` keeps compatibility wrappers and command parsing at the
   effect boundary.
4. Deferred: compand envelope/transfer extraction needs a narrower streaming
   state contract first because `compand` and `mcompand` currently share
   command-shaped configs and delay/crossover behavior.
5. Keep spectral/FFT, time-scale, resampling, and analyzer report primitives in
   their current owners until a later feature narrows dependency ownership and
   streaming-state contracts.

## Phased Plan

### Feature 7.1: DSP primitive inventory and ownership audit

Status: completed.

Audit current `auralis-effects` DSP-heavy modules and record, for each
candidate primitive:

- current owning effect module;
- known users and likely future users;
- whether it should move to `auralis-dsp`, stay effect-local, or remain blocked
  pending API stabilization;
- scalar/SIMD status and whether backend dispatch is useful;
- required tests before extraction: analytical tests, effect regression,
  SoX-ng golden coverage where applicable, chunk invariance, and
  scalar-vs-SIMD differential tests where applicable.

This feature produces the concrete extraction order. It should run after the
6.9 specialized-effect classification pass and before adding more effect
families that would copy shared algorithms into `auralis-effects`.

### Feature 7.2: first low-risk primitive extraction

Status: completed.

Extract the direct-form biquad runtime and normalized coefficient type from
`auralis-effects` into `auralis-dsp`, then re-export the public names from
`auralis-effects` so typed effect users do not see a breaking API move. Keep
SoX-ng command parsing, effect registry entries, and effect-specific coefficient
presets in `auralis-effects`. The commit must keep public APIs Auralis-owned and
must not expose implementation crate types through `auralis-core`.

The implementation also moved `BiquadWidth` and the generic RBJ cookbook helper
methods into `auralis-dsp`: once `BiquadCoefficients` is owned by `auralis-dsp`,
Rust's coherence rules require its inherent helper methods to live there too.
`auralis-effects` still converts DSP-level biquad errors into existing
`EffectError` variants at command/effect boundaries.

Required tests:

- analytical primitive tests in the new owner module;
- effect-level regression tests proving existing command behavior is unchanged;
- SoX-ng golden tests where the affected effect has comparable SoX-ng behavior;
- chunk-invariance coverage for stateful primitives;
- scalar/SIMD status, including differential tests when SIMD applies or a
  narrow documented N/A reason when it does not.

### Feature 7.3: FIR numeric primitive extraction

Status: completed.

Split the FIR numeric primitive from command-style coefficient sources. Move
the validated coefficient container and centered `FirState` into `auralis-dsp`,
while keeping `fir` command parsing, stdin/file/inline coefficient acquisition,
diagnostics, and registry entries in `auralis-effects`.

Each extraction must be small enough to review independently and must preserve
the effect command surface, typed effect APIs, diagnostics, and golden-test
expectations.

The implementation added `auralis_dsp::FirCoefficients` and
`auralis_dsp::FirState` as the owner of reusable finite-coefficient validation,
centered convolution, chunk-preserving state, and tail flushing.
`auralis-effects` now keeps a compatibility `FirCoefficients` wrapper so
`FirCoefficients::parse_text`, file/stdin source semantics, diagnostics, and
typed effect construction remain at the command/effect boundary. Its public
`FirState` wrapper delegates to the DSP primitive, preserving existing
chunk-state call sites without exposing command parsing through `auralis-dsp`.

Required tests:

- analytical primitive tests in `auralis-dsp` for non-finite rejection, empty
  null streams, SoX-ng-centered impulse alignment, and chunk equivalence;
- effect-level regression tests proving command parsing, file-backed loading,
  typed processing, and existing diagnostics are unchanged;
- existing SoX-ng golden cases for `fir`, `sinc`, `hilbert`, `loudness`, and
  other concrete FIR-backed effects remain the behavior oracle;
- SIMD status remains N/A until a deliberate vectorized convolution backend is
  introduced.

### Feature 7.4: next reusable primitive family selection

Status: completed.

Choose the next primitive family after the successful biquad and FIR boundary
moves. The next leaf should narrow whether deterministic dither/noise state or
compand envelope/transfer state has the lower-risk extraction path, then record
the selected owner, public compatibility strategy, and required tests before
moving code.

If an audited primitive has only one effect user, no foreseeable reuse, no
backend-dispatch need, and no independent analytical-test value, leave it
effect-local and record that decision instead of moving code for its own sake.

Feature 7.4 selects deterministic dither/noise as the next reusable primitive
family. The extraction has the lower-risk path because the current `Dither`
configuration and `DitherState` are compact, deterministic, chunk-invariant,
already used by both the `dither` effect and explicit output-boundary dither,
and do not depend on effect command parsing. The selected owner is
`auralis-dsp`: it should own the reusable dither configuration, deterministic
PRNG state, Shibata-style noise-shaping feedback state, normalized-to-internal
quantization helpers, and scalar processing loop.

`auralis-effects` must keep SoX-ng command parsing, option rendering,
diagnostics, registry entries, and public compatibility re-exports so existing
typed-effect users can continue to use `auralis_effects::{Dither, DitherState,
DitherMode, DitherNoiseShape, DEFAULT_DITHER_SEED}`. The high-level `auralis`
output-dither policy may either call the DSP primitive directly or continue
through a compatibility wrapper, but public APIs must remain Auralis-owned and
must not expose implementation crate details through `auralis-core`.

Compand envelope/transfer extraction is deliberately deferred. Although the
transfer curve and envelope follower are reusable, the current implementation
shares whole command-shaped `Compand` configs with `mcompand`, mixes
shared/per-channel envelope grouping with look-ahead delay buffers, and still
needs a smaller streaming-state contract before a low-risk owner move.

Required tests for the dither/noise extraction:

- analytical primitive tests in `auralis-dsp` for invalid precision,
  deterministic seed sequences, quantization bounds, non-finite input policy,
  and Shibata feedback state;
- chunk-invariance tests for plain TPDF, sloped TPDF, and Shibata-shaped
  processing through the new DSP owner;
- effect-level regression tests proving command parsing, rendering, typed
  construction, registry visibility, and chain execution are unchanged;
- `auralis` output-policy integration tests proving explicit output dither
  remains deterministic and happens at the same boundary;
- existing explicit `dither` SoX-ng goldens and automatic output-dither goldens;
- SIMD N/A documented for the stateful PRNG/noise-shaping loop unless a later
  vectorized design can prove identical state ordering.

### Feature 7.5: deterministic dither/noise primitive extraction

Status: completed.

Moved the reusable deterministic dither/noise primitive selected by Feature 7.4
into `auralis-dsp`. The new DSP owner exports `Dither`, `DitherState`,
`DitherMode`, `DitherNoiseShape`, `DitherError`, and
`DEFAULT_DITHER_SEED`, with analytical tests for invalid precision,
deterministic seed sequences, quantization bounds, sloped TPDF history,
chunk-invariant Shibata feedback, and non-finite sample handling policy.

`auralis-effects` preserves the existing public `Dither` and `DitherState`
names as compatibility wrappers, keeps `AudioBuffer` processing, SoX-ng command
parsing/rendering, diagnostics, registry behavior, and chain execution at the
effect boundary, and maps DSP precision errors back to
`EffectError::InvalidDither`. The high-level `auralis` output-dither policy
continues through that compatibility wrapper, so explicit output dither remains
deterministic at the same output boundary.

This feature must not move synth waveform generation or command-specific
dither options into `auralis-dsp`; those remain separate boundary concerns.
SIMD remains N/A for this primitive because the PRNG sequence and
noise-shaping feedback state make sample order part of the observable output.
