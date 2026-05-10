# 7.x Reusable DSP Primitives Roadmap

This is a future-plan track for extracting reusable DSP kernels after the
current specialized-effect classification work. It does not reopen completed
2.x, 4.x, or 6.1-6.8 milestone plans.

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

## Phased Plan

### Feature 7.1: DSP primitive inventory and ownership audit

Status: planned.

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

Status: blocked until Feature 7.1 records the first candidate.

Extract one low-risk, already-tested primitive from an effect-local module into
`auralis-dsp` or explicitly record why it remains effect-local. The commit must
keep public APIs Auralis-owned and must not expose implementation crate types
through `auralis-core`.

Required tests:

- analytical primitive tests in the new owner module;
- effect-level regression tests proving existing command behavior is unchanged;
- SoX-ng golden tests where the affected effect has comparable SoX-ng behavior;
- chunk-invariance coverage for stateful primitives;
- scalar/SIMD status, including differential tests when SIMD applies or a
  narrow documented N/A reason when it does not.

### Feature 7.3: repeated extraction leaves

Status: planned after Feature 7.2.

Continue one primitive family per gnhf iteration. Each extraction must be small
enough to review independently and must preserve the effect command surface,
typed effect APIs, diagnostics, and golden-test expectations.

If an audited primitive has only one effect user, no foreseeable reuse, no
backend-dispatch need, and no independent analytical-test value, leave it
effect-local and record that decision instead of moving code for its own sake.
