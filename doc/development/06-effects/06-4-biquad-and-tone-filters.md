---
kind: historical-roadmap
status: superseded
superseded_by:
  - ../effects.md
  - ../dsp.md
  - ../oracles.md
  - ../testing.md
---

## Milestone 6.4: biquad and tone filters

This milestone record is retained for implemented effect and primitive history
and coverage evidence. New effect, DSP, or oracle planning belongs in the flat
short documents under `../effects/`, `../dsp/`, and `../oracles/`.

### Feature 6.4.1: biquad primitive

Implement a scalar biquad primitive before exposing effect names.

Status: implemented.

Implementation notes:

- Added reusable `BiquadCoefficients`, `Biquad`, and `BiquadState` types for
  normalized direct-form biquad filtering with `a0 = 1`.
- Raw `b0 b1 b2 a0 a1 a2` coefficient input can be normalized through the
  primitive while rejecting non-finite coefficients and zero `a0` values.
- Processing uses deterministic scalar transposed direct-form II state with
  one independent state per channel; callers can preserve `BiquadState` across
  chunks for streaming-safe processing.
- Coverage includes analytical impulse-response, identity, raw-normalization,
  invalid-coefficient, stereo-state, property, and chunk-invariance tests. No
  SoX-ng golden command is added yet because Feature 6.4.2 owns the exposed
  `biquad` effect surface.

### Feature 6.4.2: `biquad` effect

Expose direct coefficient-based biquad processing.

Status: implemented.

Implementation notes:

- `biquad b0 b1 b2 a0 a1 a2` now parses through the shared effect command
  model, normalizes raw coefficients by `a0`, rejects non-finite coefficients
  and zero `a0`, and renders canonical equivalent commands with `a0 = 1`.
- Effect-chain execution applies the existing scalar biquad primitive with
  independent state per channel.
- Golden coverage includes mono impulse and stereo sine one-pole cases against
  SoX-ng, with parser fuzz coverage and L0-L7 matrix entries for the exposed
  command surface.

### Feature 6.4.3: RBJ coefficient helpers

Add reusable coefficient helpers for tone filters.

Status: implemented.

Implementation notes:

- `BiquadCoefficients` now exposes RBJ cookbook helpers from a focused
  `biquad_design` module for low-pass,
  high-pass, constant-skirt and constant-peak band-pass, band-reject/notch,
  two-pole all-pass, peaking EQ, low shelf, and high shelf filters.
- `BiquadWidth` models SoX-ng-compatible width units for future command
  surfaces: quality factor, octave bandwidth, hertz bandwidth, kilohertz
  bandwidth, and shelf slope.
- Helper validation rejects non-finite sample rates, frequencies, widths, and
  gains; frequencies at or above Nyquist; non-positive widths; and shelf slope
  values outside SoX-ng's `0 < slope <= 1` range.
- Coverage includes analytical coefficient checks for every helper family,
  width-unit equivalence coverage, and invalid-design rejection tests. No
  SoX-ng golden command surface is added in this feature because subsequent
  6.4.x effects own the user-facing commands.

### Feature 6.4.4: `allpass`

Status: implemented.

Implementation notes:

- Added a typed `AllPass` effect with SoX-ng-compatible default RBJ
  `allpass frequency width`, `allpass -1 frequency`, and `allpass -2
  frequency` forms.
- Command parsing supports hertz, kilohertz, Q, and octave width suffixes,
  including SoX-ng-style hertz default for unsuffixed width values and
  kilohertz frequency shorthand.
- Processing delegates to the scalar biquad primitive with independent
  per-channel state; runtime validation rejects frequencies at or above
  Nyquist for the input sample rate.
- Coverage includes analytical coefficient tests, command and chain integration
  tests, L4 finite-output property coverage, L5 state-preserving chunk
  invariance, parser fuzz seeds, and standalone SoX-ng golden cases for the
  default RBJ and `-1` forms.

### Feature 6.4.5: `band`

Status: implemented.

Implementation notes:

- Added a typed `Band` effect matching SoX-ng's historical resonator
  `band [-n] frequency [width]` command family.
- Width defaults to `frequency / 2` and accepts hertz, kilohertz, Q, and octave
  units; `-n` selects SoX-ng's alternate unpitched/noise scaling.
- Processing delegates to the scalar stateful biquad primitive with independent
  per-channel state, so chunked processing is exact when callers preserve
  `BiquadState` per channel.
- Coverage includes analytical coefficient tests, command and chain integration
  tests, L4 finite-output property coverage, L5 state-preserving chunk
  invariance, parser fuzz seeds, and standalone SoX-ng golden cases for the
  default and `-n` forms.

### Feature 6.4.6: `bandpass`

Status: implemented.

Implementation notes:

- Added a typed `BandPass` effect matching SoX-ng's RBJ `bandpass [-c]
  frequency width` command family.
- The default form uses constant 0 dB peak gain, while `-c` selects
  constant-skirt gain. Width accepts hertz, kilohertz, Q, and octave suffixes.
- Processing delegates to the scalar stateful biquad primitive with independent
  per-channel state, so chunked processing is exact when callers preserve
  `BiquadState` per channel.
- Coverage includes analytical coefficient tests, command and chain integration
  tests, L4 finite-output property coverage, L5 state-preserving chunk
  invariance, parser fuzz seeds, and standalone SoX-ng golden cases for the
  default and `-c` forms.

### Feature 6.4.7: `bandreject`

Status: implemented.

Implementation notes:

- Added a typed `BandReject` effect matching SoX-ng's RBJ `bandreject
  frequency width` command family.
- Processing delegates to the scalar stateful biquad primitive with independent
  per-channel state, so chunked processing is exact when callers preserve
  `BiquadState` per channel.
- Width accepts hertz, kilohertz, Q, and octave suffixes; invalid slope widths,
  non-positive frequencies, and frequencies at or above Nyquist are rejected as
  invalid biquad designs.
- Coverage includes analytical coefficient tests, command and chain integration
  tests, L4 finite-output property coverage, L5 state-preserving chunk
  invariance, parser fuzz seeds, and standalone SoX-ng golden cases for mono and
  stereo input.

### Feature 6.4.8: `bass`

Status: implemented.

Implementation notes:

- Added a typed `Bass` effect matching SoX-ng's RBJ low-shelf `bass gain
  [frequency [width]]` command family, including the 100 Hz and `0.5s` command
  defaults.
- Width accepts shelf slope plus hertz, kilohertz, Q, and octave forms; invalid
  slope widths above 1, non-positive frequencies, and frequencies at or above
  Nyquist are rejected as invalid biquad designs.
- Processing delegates to the scalar stateful biquad primitive with independent
  per-channel state, so chunked processing is exact when callers preserve
  `BiquadState` per channel.
- Coverage includes analytical coefficient tests, command and chain integration
  tests, L4 finite-output property coverage, L5 state-preserving chunk
  invariance, parser fuzz seeds, and standalone SoX-ng golden cases for the
  default and explicit-Q forms. Tolerance follows the existing one-PCM16-LSB
  filter-effect policy because the RBJ coefficients match SoX-ng and residual
  differences are from sample quantization.

### Feature 6.4.9: `treble`

Status: implemented.

Implementation notes:

- Added a typed `Treble` effect matching SoX-ng's RBJ high-shelf `treble gain
  [frequency [width]]` command family, including the 3000 Hz and `0.5s`
  command defaults.
- Width accepts shelf slope plus hertz, kilohertz, Q, and octave forms; invalid
  slope widths above 1, non-positive frequencies, and frequencies at or above
  Nyquist are rejected as invalid biquad designs.
- Processing delegates to the scalar stateful biquad primitive with independent
  per-channel state, so chunked processing is exact when callers preserve
  `BiquadState` per channel.
- Coverage includes analytical coefficient tests, command and chain integration
  tests, L4 finite-output property coverage, L5 state-preserving chunk
  invariance, parser fuzz seeds, and standalone SoX-ng golden cases for the
  default and explicit-Q forms. Tolerance follows the existing one-PCM16-LSB
  filter-effect policy because the RBJ coefficients match SoX-ng and residual
  differences are from sample quantization.

### Feature 6.4.10: `equalizer`

Status: implemented.

Implementation notes:

- Added a typed `Equalizer` effect matching SoX-ng's RBJ peaking-EQ
  `equalizer frequency width gain` command shape.
- Width accepts hertz, kilohertz, Q, and octave forms; shelf slope is rejected
  because the peaking-EQ helper does not use shelf slope semantics.
- Processing delegates to the scalar stateful biquad primitive with independent
  per-channel state, so chunked processing is exact when callers preserve
  `BiquadState` per channel.
- Coverage includes analytical coefficient tests, command and chain integration
  tests, L4 finite-output property coverage, L5 state-preserving chunk
  invariance, parser fuzz seeds, and standalone SoX-ng golden cases for Q-width
  mono and octave-width stereo forms. Tolerance follows the same one-PCM16-LSB
  filter-effect policy as the other RBJ filters.

### Feature 6.4.11: `lowpass`

Status: implemented.

Implementation notes:

- Added a typed `LowPass` effect matching SoX-ng's `lowpass [-1|-2]
  frequency [width]` command family. The default and `-2` forms use the RBJ
  two-pole low-pass shape, with omitted width defaulting to Butterworth
  `0.707q`; `-1` uses SoX-ng's single-pole RC-style low-pass formula.
- Width accepts hertz, kilohertz, Q, and octave forms; shelf slope is rejected
  because low-pass filters do not use shelf slope semantics.
- Processing delegates to the scalar stateful biquad primitive with independent
  per-channel state, so chunked processing is exact when callers preserve
  `BiquadState` per channel.
- Coverage includes analytical coefficient tests, command and chain integration
  tests, L4 finite-output property coverage, L5 state-preserving chunk
  invariance, parser fuzz seeds, and standalone SoX-ng golden cases for the
  default two-pole and one-pole forms. Tolerance follows the same one-PCM16-LSB
  filter-effect policy as the other RBJ filters and single-pole biquad forms.

### Feature 6.4.12: `highpass`

Status: implemented.

Implementation notes:

- Added a typed `HighPass` effect matching SoX-ng's `highpass [-1|-2]
  frequency [width]` command family. The default and `-2` forms use the RBJ
  two-pole high-pass shape, with omitted width defaulting to Butterworth
  `0.707q`; `-1` uses SoX-ng's single-pole RC-style high-pass formula.
- Width accepts hertz, kilohertz, Q, and octave forms; shelf slope is rejected
  because high-pass filters do not use shelf slope semantics.
- Processing delegates to the scalar stateful biquad primitive with independent
  per-channel state, so chunked processing is exact when callers preserve
  `BiquadState` per channel.
- Coverage includes analytical coefficient tests, command and chain integration
  tests, L4 finite-output property coverage, L5 state-preserving chunk
  invariance, parser fuzz seeds, and standalone SoX-ng golden cases for the
  default two-pole and one-pole forms. Tolerance follows the same one-PCM16-LSB
  filter-effect policy as the other RBJ filters and single-pole biquad forms.

### Feature 6.4.13: `deemph`

Status: implemented.

Implementation notes:

- Added a typed no-argument `Deemph` effect matching SoX-ng's fixed CD/DAT
  de-emphasis presets: 44.1 kHz uses 5283 Hz, `0.4845s`, and -9.477 dB; 48 kHz
  uses 5356 Hz, `0.479s`, and -9.62 dB. Other sample rates return an invalid
  biquad design error, matching SoX-ng's accepted input-rate restriction.
- Processing delegates to the scalar stateful high-shelf biquad primitive, with
  independent per-channel state and exact state-preserving chunk equivalence.
- Coverage includes coefficient and command tests, chain integration, L4
  finite-output property coverage, L5 chunk invariance, parser fuzz seed
  coverage, and standalone SoX-ng golden cases for mono and stereo 48 kHz L0
  corpora. Tolerance follows the one-PCM16-LSB filter-effect policy used by the
  surrounding biquad-backed filters.

### Feature 6.4.14: `riaa`

Status: implemented.

Implementation notes:

- Added a typed no-argument `Riaa` effect matching SoX-ng's fixed RIAA playback
  equalization roots for 44.1 kHz, 48 kHz, 88.2 kHz, 96 kHz, and 192 kHz, with
  the same 1 kHz response normalization and unsupported-rate rejection.
- Processing delegates to the scalar stateful biquad primitive, with
  independent per-channel state and exact state-preserving chunk equivalence.
- Coverage includes coefficient and command tests, chain integration, L4
  finite-output property coverage, L5 chunk invariance, parser fuzz seed
  coverage, and standalone SoX-ng golden cases for mono and stereo 48 kHz L0
  corpora. Tolerance follows the one-PCM16-LSB filter-effect policy used by the
  surrounding biquad-backed filters.
