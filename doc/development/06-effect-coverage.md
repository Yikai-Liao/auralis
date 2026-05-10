# 6.x Effect Coverage Milestone

Implement effects in the order below. Each effect must satisfy the effect test
contract from [`03-test-infrastructure.md`](03-test-infrastructure.md), update
the layered coverage matrix from Feature 5.7.7, include scalar and SIMD work
where applicable, and update the SoX-ng coverage entry.

## Milestone 6.1: complete existing SoX-ng semantics

### Feature 6.1.1: `gain` headroom and reclaim options

Status: implemented.

Implement `gain -h` and `gain -r`.

Acceptance tests:

- manifest case per option;
- option interactions tested where SoX-ng documents combinations;
- scalar-vs-SIMD tests for gain kernels;
- SoX-ng golden comparisons.

Implementation notes:

- `Gain` now carries an explicit SoX-ng headroom mode while preserving plain
  fixed-gain processing for direct typed API calls.
- The effect command parser accepts `gain -h`, `gain -r`, and combined
  `gain -rh`/`gain -hr` option forms with an optional fixed gain value, while
  normalization, limiter, and channel-balancing gain options remain scheduled
  for later 6.1 features.
- `EffectChain` tracks reclaimable headroom metadata between commands:
  `gain -h DB` applies the fixed gain and records the reserved multiplier, and
  a later `gain -r` scans the current buffer and restores only as much as can
  fit below full scale.
- Golden coverage includes a standalone `gain -h` case and a positional
  `gain -h ... gain -r` chain case against SoX-ng.

### Feature 6.1.2: `gain` normalize and limiter options

Status: implemented.

Implement remaining level-management options that belong to `gain`, without
confusing them with pipeline `--norm`.

Implementation notes:

- The effect command parser accepts `gain -n`, `gain -l`, and combined
  `gain -nl`/`gain -ln` option forms with an optional fixed gain value.
- `gain -n DB` scans the current chain buffer, scales the peak to full scale,
  then applies the fixed dB offset. Silence remains silent.
- `gain -l DB` applies SoX-ng's simple limiter curve after the fixed gain; the
  limiter is scalar because it is not a pure multiply kernel.
- SoX-ng's mutually exclusive combinations are rejected: `-n` with `-r`, and
  `-l` with `-h`. Channel equalize and balance options remain scheduled for
  Feature 6.1.3.
- Golden coverage includes standalone `gain -n` and `gain -l` cases against
  SoX-ng.

### Feature 6.1.3: `gain` channel equalize and balance options

Status: implemented.

Implement channel-aware gain semantics and document differences from output
channel policies.

Implementation notes:

- The effect command parser accepts `gain -e`, `gain -B`, and `gain -b`,
  including combined forms such as `gain -Bn`.
- `gain -e DB` scans each channel peak and scales quieter channels to the
  largest channel peak before applying the fixed gain.
- `gain -B DB` scans per-channel RMS and balances quieter channels to the
  largest channel RMS without clip protection.
- `gain -b DB` uses the same RMS balancing but attenuates all balanced channels
  if needed to keep the pre-fixed-gain balanced peak within full scale; a
  positive fixed gain can still exceed full scale, matching SoX-ng.
- `gain -B -n DB` normalizes the balanced result before applying fixed gain and
  matches SoX-ng's `-Bn`/`-bn` behavior.
- These modes are channel-local `gain` effects and do not change channel count,
  unlike output channel policies or future explicit channel-routing effects.
- SoX-ng's mutually exclusive mode group is rejected: only one of `-e`, `-B`,
  `-b`, and `-r` may be given.
- Golden coverage includes standalone `gain -e`, `gain -B`, and `gain -b`
  stereo cases against SoX-ng.

### Feature 6.1.4: `fade` curve types

Status: implemented.

Add supported SoX-ng fade curve families with analytical and golden coverage.

Implementation notes:

- The typed fade model now carries a `FadeCurve` with SoX-ng `q`, `h`, `l`,
  `t`, and `p` variants; `Fade::new` remains the existing linear constructor
  and command parsing without an explicit type follows SoX-ng's logarithmic
  default.
- Linear `t` fades continue to use the backend-dispatched fade kernel. The
  non-linear curves use deterministic scalar coefficient evaluation and fall
  back to that scalar path even when SIMD is requested.
- Analytical tests cover the SoX-ng coefficient formulas for quarter-sine,
  half-sine, logarithmic, linear, and inverted-parabola fade-in curves.
- Golden coverage includes standalone fade-in cases for all five curve
  families. Fade-out positional semantics remain scheduled for Feature 6.1.5.

### Feature 6.1.5: `fade` stop position and fade-out length

Status: implemented.

Complete SoX-ng positional fade semantics beyond the initial frame-count subset.

Implementation notes:

- `fade [type] fade-in-length` remains a fade-in-only command.
- `fade [type] fade-in-length stop-position` now enables SoX-ng fade-out
  processing and defaults the fade-out length to the fade-in length.
- `fade [type] fade-in-length stop-position fade-out-length` supports explicit
  fade-out length; `0` and `-0` stop positions mean the end of the input.
- Command-style fades truncate at the stop position or pad silence when the
  stop is past the input length, and use SoX-ng's fade-out endpoint indexing
  where the final retained frame is attenuated by `1 / fade-out-length`.
- Direct `Fade::new(fade_in, fade_out)` preserves Auralis' original in-place
  end-fade behavior for typed API and legacy CLI flag callers.
- Golden coverage includes fade-out-at-end and explicit stop-position cases.

### Feature 6.1.6: `dcshift` limiter gain

Status: implemented.

Implement the limiter gain option and document clipping/limiting behavior.

Implementation notes:

- `DcShift` now carries an optional SoX-ng limiter gain while preserving the
  existing single-argument direct API behavior.
- `dcshift SHIFT LIMITER_GAIN` parses and renders through the shared typed
  command model, including flat effect-chain token streams.
- Plain `dcshift` remains an unclipped additive offset inside Auralis; the
  limiter-gain form follows SoX-ng's peak-threshold limiter and clips the
  effect output immediately before later chain commands see it.
- Analytical tests cover positive and negative limiter behavior, command
  parsing, and scalar-vs-SIMD fallback parity. Golden coverage includes a
  standalone full-scale mono limiter case against SoX-ng.

### Feature 6.1.7: `pad` positioned padding

Status: implemented.

Support SoX-ng-style positioned padding, not only start/end padding.

Implementation notes:

- The typed pad model now carries optional sorted `PositionedPad` insertions in
  addition to the existing start/end padding fields.
- The effect command parser accepts `pad LENGTH@POSITION`, multiple ascending
  positioned insertions, and `@-0` end-position padding while preserving the
  existing `pad START END` start/end form.
- Pad processing inserts silence before the requested input frame, rejects
  duplicate or unsorted positions, and returns a typed error when a position is
  after the input duration.
- Golden coverage includes standalone mono and stereo positioned-pad cases
  against SoX-ng, and L7 fuzz seeds cover the positioned command syntax.

### Feature 6.1.8: `trim` multiple and relative positions

Status: implemented.

Support multiple trim ranges and relative-position forms.

Implementation notes:

- `Trim` now stores SoX-ng-style positions that alternate between discarding
  and copying audio, while `Trim::new(start, end)` preserves the existing
  single-range typed API.
- The command parser accepts open-ended `trim START`, relative length
  positions, absolute `=POSITION` resumes, end-relative `-POSITION`, and `-0`
  end-of-input forms.
- Standalone golden coverage includes multi-range mono trimming and an
  absolute-resume stereo case against SoX-ng. The existing middle-range
  goldens now use SoX-ng's start-plus-length command form on both Auralis and
  SoX-ng sides.

## Milestone 6.2: volume, level, and simple modulation effects

### Feature 6.2.1: `vol`

Status: implemented.

Implement volume scaling and supported option syntax.

Implementation notes:

- Added a typed `Vol` effect with amplitude, power, and dB gain modes, including
  negative amplitude/power phase inversion and immediate SoX-ng-style clipping.
- The effect command parser accepts `vol GAIN`, `vol GAIN amplitude`,
  `vol GAIN power`, `vol GAIN dB`, suffix forms such as `vol -6dB`, and
  optional limiter gain when the absolute amplitude multiplier is at least one.
- Limiter processing follows SoX-ng's threshold formula and remains scalar;
  the no-limiter multiply path uses the selected gain backend and clips after
  multiplication.
- Golden coverage includes standalone amplitude, dB, power, and limiter-gain
  cases against SoX-ng, with L4 identity/finite-output coverage, L5 chunk
  invariance, L6 scalar-vs-SIMD parity, and an L7 fuzz seed for limiter syntax.

### Feature 6.2.2: `norm`

Status: implemented.

Implement effect-level normalization semantics separately from output `--norm`.

Implementation notes:

- Added a typed `Norm` effect with optional target dBFS level, defaulting to
  0 dBFS, and wired it through command parsing, registry resolution, and
  effect-chain execution.
- `norm [level]` follows SoX-ng's current shim semantics for `gain -n [level]`:
  scan the whole effect input, leave silence unchanged, reject non-finite
  samples with a typed error, and scale the peak to the requested target.
- The effect is intentionally separate from output `--norm`; `Norm` runs at its
  position in an `EffectChain`, while output normalization remains a final
  write policy.
- Golden coverage includes standalone default and target-level norm cases
  against SoX-ng, with L4 finite/silence coverage, L6 scalar-vs-SIMD parity for
  the multiply pass, and an L7 fuzz seed for norm command parsing. L5 chunk
  invariance is not applicable because norm is a whole-buffer scan.

### Feature 6.2.3: `contrast`

Status: implemented.

Implement contrast enhancement with analytical and golden coverage.

Implementation notes:

- Added a typed `Contrast` effect implementing SoX-ng's phase contrast formula
  with the documented `0..=100` amount range and default amount `75`.
- The effect command parser accepts `contrast` and `contrast AMOUNT`, renders
  the default explicitly as `contrast 75`, and rejects invalid amounts with a
  typed error.
- Golden coverage includes standalone mono default and stereo explicit-amount
  cases against SoX-ng, with L4 finite-output coverage, L5 chunk invariance,
  and an L7 fuzz seed. L6 SIMD is documented as not applicable because the
  processor is a scalar transcendental sine transform.

### Feature 6.2.4: `softvol`

Status: implemented.

Implement soft volume scaling if SoX-ng semantics are well-defined enough for
golden coverage.

Implementation notes:

- Added a typed `SoftVol` effect with initial volume, double-time recovery, and
  headroom settings matching SoX-ng's `softvol [volume [double-time
  [headroom]]]` command shape.
- Processing scans each frame across channels, lowers the current multiplier
  before output when that frame would exceed the headroom-adjusted maximum, and
  optionally recovers upward after each frame according to the configured
  doubling time and input sample rate.
- The effect command parser accepts default, partial, and full softvol argument
  lists, renders explicit defaults, and rejects negative or non-finite values
  with a typed error.
- Golden coverage includes standalone mono fixed-volume and stereo
  recovery/headroom cases against SoX-ng, with L4 finite-output coverage, L5
  stateful chunk coverage, and an L7 fuzz seed for softvol command parsing.

### Feature 6.2.5: `tremolo`

Status: implemented.

Implement deterministic tremolo modulation.

Implementation notes:

- Added a typed `Tremolo` effect matching SoX-ng's `tremolo speed [depth]`
  command shape, with speed in hertz and depth in `(0, 100]`, defaulting to
  `40`.
- Processing follows SoX-ng's `synth sine fmod` mapping: the envelope starts at
  full volume and ranges from `1 - depth / 100` to `1`, with the frame phase
  derived from the input sample rate.
- The effect command parser accepts required speed plus optional depth, renders
  explicit defaults, and rejects negative, zero-depth, over-100, or non-finite
  values with a typed error.
- Golden coverage includes standalone mono default-depth and stereo
  explicit-depth cases against SoX-ng, with L4 finite-output coverage, L5
  frame-offset chunk coverage, and an L7 fuzz seed for tremolo command parsing.

### Feature 6.2.6: `overdrive`

Status: implemented.

Implement overdrive with documented transfer function and clipping behavior.

Implementation notes:

- Added a typed `Overdrive` effect matching SoX-ng's `overdrive [gain [color]]`
  command shape, with gain and color in the documented `0..=100` range and
  defaults of `20`.
- Processing follows SoX-ng's driven cubic soft-clip transfer, `color / 200`
  bias, and channel-local high-pass output state; `overdrive 0` remains a null
  effect in the non-keymapped command path.
- Golden coverage includes standalone mono default and stereo explicit-argument
  cases against SoX-ng, with L4 finite-output coverage, L5 stateful chunk
  coverage, and an L7 fuzz seed for overdrive command parsing.

### Feature 6.2.7: `saturation`

Status: implemented.

Implement saturation with documented transfer function and golden coverage.

Implementation notes:

- Added a typed `Saturation` effect covering SoX-ng's `tanh`, `sqrt`, and
  `diode` transfer families with `blend`, `offset`, and type-specific
  `drive`/`color`/`threshold` parameters.
- Processing recenters the wet transfer around zero input, applies SoX-ng's
  safety output-gain compensation, mixes wet and dry paths, and clips to the
  normalized sample range inside the effect.
- The effect command parser accepts `saturation [type [blend [offset
  [drive|color|threshold]]]]`, renders explicit defaults, and rejects invalid
  ranges with typed errors.
- Golden coverage includes standalone mono default `tanh` and stereo explicit
  `sqrt` cases against SoX-ng, with L4 finite-output coverage, L5
  chunk-invariance coverage, and an L7 fuzz seed for saturation command
  parsing. L6 SIMD is not applicable because this is a scalar nonlinear
  transform.

### Feature 6.2.8: `repeat`

Status: implemented.

Implement deterministic repeat semantics and output-length validation.

Implementation notes:

- Added a typed `Repeat` effect matching SoX-ng's finite `repeat [count]`
  command shape: output contains the original input plus `count` additional
  copies, the default count is `1`, and count `0` is an identity transform.
- `Repeat` rejects counts above SoX-ng's finite `UINT_MAX - 1` range and
  deliberately rejects SoX-ng's indefinite `repeat -` form because Auralis'
  current in-memory processing requires bounded output.
- Processing preserves planar channel grouping and validates output frame and
  sample allocation sizes before constructing the repeated buffer.
- Golden coverage includes standalone mono explicit-count and stereo default
  repeat cases against SoX-ng, with L4 identity/finite-output coverage and an
  L7 fuzz seed for repeat command parsing. L5 and L6 are not applicable because
  repeat is a whole-buffer structural duplication without a SIMD kernel.

## Milestone 6.3: channel and mixing effects

### Feature 6.3.1: `channels`

Status: implemented.

Implement the explicit `channels` effect. It should share conversion primitives
with output channel policy without hiding behavior.

Implementation notes:

- Added a typed `Channels` effect matching SoX-ng's `channels number` command
  shape. It preserves sample rate, sample format, and frame count while
  changing decoded channel layout.
- Matching channel counts are identity copies, upmixing duplicates input
  channels round-robin, and downmixing averages deterministic input-channel
  groups using the same backend-dispatched primitive now shared by the output
  channel policy.
- Golden coverage includes standalone mono-to-stereo and stereo-to-mono cases
  against SoX-ng. L4 covers identity and finite-output behavior, L6 compares
  scalar and requested-SIMD downmix execution, and L7 includes a parser fuzz
  seed for `channels`.

### Feature 6.3.2: `remix` basic routing

Status: implemented.

Implement basic channel routing.

Implementation notes:

- Added a typed `Remix` effect for SoX-ng-style basic out-spec routing with
  1-based channel numbers, comma-separated contributors, channel ranges,
  open ranges, the `-` all-channel range, and standalone `0` silent outputs.
- Multi-input output specs use SoX-ng's default `1 / n` scaling when no source
  gain modifier is present. Feature 6.3.3 extends this same model with gain
  modifiers and level-scaling options.
- Chain execution treats `remix` as a whole-buffer channel-shape transform and
  reports out-of-bounds input channels as typed command-context failures.
- Golden coverage includes mono silent/copy routing and stereo mixdown cases
  against SoX-ng, with L4 identity/finite-output coverage and an L7 fuzz seed.
  L5 and L6 are not applicable because remix is a structural channel
  routing/mixing transform without a SIMD kernel.

### Feature 6.3.3: `remix` gain modifiers

Status: implemented.

Add gain modifiers and option interactions.

Implementation notes:

- `RemixOutputSpec` now stores optional per-source gain modifiers alongside
  source routing specs, covering SoX-ng's `v` voltage multiplier, `p` power-dB
  multiplier, and `i` inverted power-dB multiplier forms.
- The `remix` command parser accepts `-a` automatic scaling, `-m` manual
  scaling, and `-p` power scaling in SoX-ng order. Default semi-automatic mode
  applies `1 / n` scaling only to output specs with no explicit gain modifier;
  `-p` changes automatic scaling to `1 / sqrt(n)`.
- Chain execution clips remixed samples to the normalized full-scale range
  before later commands see them, matching SoX-ng's effect-local clipping.
- Golden coverage includes stereo source-gain and automatic power-scaling cases
  against SoX-ng, with analytical/parser coverage and an updated L7 fuzz seed.

### Feature 6.3.4: `swap`

Status: implemented.

Implement channel swapping.

Implementation notes:

- Added a typed `Swap` effect matching SoX-ng's adjacent channel-pair swap:
  stereo channels exchange positions, multichannel input swaps pairs
  `1 <-> 2`, `3 <-> 4`, and an odd trailing channel is preserved.
- The `swap` command parser accepts no arguments, renders as `swap`, and is
  wired through the effect registry and effect-chain execution path.
- Golden coverage includes mono identity and stereo pair-swap cases against
  SoX-ng, with L4 swap-twice identity coverage, frame-chunk invariance, and an
  L7 fuzz seed for command parsing.

### Feature 6.3.5: `oops`

Status: implemented.

Implement out-of-phase stereo behavior.

Implementation notes:

- Added a typed `Oops` effect matching SoX-ng's `oops` alias for
  `remix 1,2i 1,2i`: subtract channel 2 from channel 1, clip to normalized
  full scale, and emit the same difference in both output channels.
- Inputs with fewer than two channels are rejected, matching SoX-ng's
  too-few-input-channels behavior; extra input channels are ignored.
- Golden coverage includes a standalone stereo `oops` case against SoX-ng, with
  L4 finite-output coverage, L5 frame-chunk coverage, and an L7 fuzz seed for
  command parsing. Mono golden coverage is intentionally not applicable because
  SoX-ng rejects mono `oops` input.

### Feature 6.3.6: `centercut` core

Implement the core center-cut algorithm.

Status: implemented.

Implementation notes:

- Added a typed `Centercut` processor that requires exactly stereo input and
  emits three planar output channels: left residual, right residual, and
  extracted center.
- The core processor uses overlapping spectral windows to estimate the shared
  center component from left/right sum and difference energy, then subtracts
  that estimate from the decoded stereo input.
- This feature intentionally does not expose the `centercut` command parser or
  SoX-ng options yet; Feature 6.3.7 owns `-a`, `-b`, `-w`, command integration,
  and standalone golden coverage.
- Analytical tests cover channel shape, centered stereo extraction,
  opposite-phase side preservation, and non-stereo rejection.

### Feature 6.3.7: `centercut` options

Complete exposed options and golden coverage.

Status: implemented.

Implementation notes:

- The typed `Centercut` processor now carries SoX-ng-style `-a` output gain,
  `-b` bass-to-sides routing below 200 Hz, and `-w` spectral window size
  options, with validation for finite gain and power-of-two window sizes in
  `8..=32768`.
- The `centercut` command parser accepts split and attached `-a`/`-w` values,
  renders canonical option tokens, and rejects unsupported options or
  non-stereo input with typed errors.
- Effect chains and CLI positional chains can execute `centercut`, producing
  three-channel left residual, right residual, and center output.
- Standalone SoX-ng golden coverage includes the bare command and an option
  case covering `-a`, `-b`, and `-w`; mono golden coverage is not applicable
  because SoX-ng rejects non-stereo input.

## Milestone 6.4: biquad and tone filters

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

## Milestone 6.5: delay, echo, and modulation effects

### Feature 6.5.1: `delay`

Status: implemented.

Implementation notes:

- Added a typed `Delay` effect matching SoX-ng's per-channel `delay
  {position}` command family. Positions can be absolute from the start,
  relative to the previous channel position, or relative to the end of the
  input; frame-count `s` positions and seconds-based time positions resolve
  against the input sample rate at processing time.
- Processing delays each configured channel independently, leaves channels
  without an explicit position undelayed, and extends every channel to the
  largest resolved delay with trailing silence. Latency is the per-channel
  resolved delay, tail flush emits the buffered original channel samples plus
  any max-delay alignment silence, and chunked streaming would be exact when a
  future streaming API preserves per-channel delay buffers and flushes tails.
- Coverage includes typed processor tests, command and chain integration tests,
  L4 finite-output property coverage, parser fuzz seeds, and standalone
  SoX-ng golden cases for mono frame delay and stereo per-channel delay.

### Feature 6.5.2: `echo`

Status: implemented.

Implementation notes:

- Added a typed `Echo` effect matching SoX-ng's parallel `echo gain-in
  gain-out <delay decay>` command family. Delay values are milliseconds,
  resolve against the input sample rate by truncating to a frame count, and
  each delay-decay pair reads from the original input delay line rather than
  feeding later taps.
- Processing applies clean input gain, sums delayed tap decays, applies final
  output gain, clips inside the effect, and extends output by the largest
  resolved delay. Latency is the largest configured tap delay; tail flush emits
  the remaining delayed samples with zero input. Chunked streaming would be
  exact when a future streaming API preserves the circular delay buffer and
  exposes an explicit tail flush.
- Coverage includes typed processor tests, command and chain integration tests,
  L4 finite-output property coverage, parser fuzz seeds, and standalone
  SoX-ng golden cases for mono delayed echo and stereo zero-delay mixing.

### Feature 6.5.3: `echos`

Status: implemented.

Implementation notes:

- Added a typed `Echos` effect matching SoX-ng's cascaded `echos gain-in
  gain-out <delay decay>` command family. Delay values are milliseconds,
  resolve against the input sample rate by truncating to a frame count, and
  each delay line feeds the next delay line plus the current input, so later
  taps include echoes of earlier echoes.
- Processing applies clean input gain, sums cascaded delayed tap decays,
  applies final output gain, clips inside the effect, and extends output by
  the sum of all resolved tap delays. Each delay must resolve to at least one
  frame, and decay is restricted to SoX-ng's `0..=1` range.
- Latency is governed by the cascaded delay sequence; tail flush emits the
  remaining cascaded delay-line contents with zero input. Chunked streaming
  would be exact when a future streaming API preserves all per-channel delay
  buffers and exposes explicit tail flush.
- Coverage includes typed processor tests, command and chain integration
  tests, L4 finite-output property coverage, parser fuzz seeds, and standalone
  SoX-ng golden cases for mono cascaded echos and stereo single-tap echos.

### Feature 6.5.4: `chorus` core

Status: implemented.

Implementation notes:

- Added a typed `Chorus` processor and `ChorusStage` configuration for the
  scalar single-stage sine-modulated delay core. The processor applies
  gain-in, one delayed stage scaled by decay, gain-out, full-scale clipping,
  and output tail extension by the maximum resolved delay.
- Delay and depth are configured in milliseconds and resolved against the
  input sample rate at processing time. The core rejects non-finite values,
  out-of-range gains/decay, negative timing parameters, zero resolved delay
  lines, zero modulation speed, and modulation speeds above the sample rate.
- Latency is the current modulated delay, and tail behavior is explicit:
  whole-buffer processing extends the output by `ceil(delay + depth)` frames.
  A future streaming API can be exact only if it preserves the per-channel
  delay line, modulation phase, and exposes a final zero-input flush.
- Coverage includes analytical typed-processor tests and L4 finite-output
  property coverage. Feature 6.5.5 later added the SoX-ng command parser,
  golden tests, interpolation options, waveform selection, and multi-delay
  command surface.

### Feature 6.5.5: `chorus` interpolation and multi-delay options

Status: implemented.

Implementation notes:

- Extended the typed `Chorus` model with SoX-ng's global `-n`, `-l`, and
  `-q` interpolation modes, sine/triangle waveform selection, per-stage wave
  overrides, and multiple stage support while preserving the single-stage
  constructor from Feature 6.5.4.
- Wired `chorus [-n|-l|-q] [-s|-t] [gain-in [gain-out [delay decay speed depth
  [-sine|-triangle]]...]]` through the effect registry, typed command parser,
  effect-chain execution, effects-file diagnostics, CLI positional chain path,
  and parser fuzz corpus.
- Processing keeps independent per-channel delay lines for each stage, sums
  the configured stage decays, clips inside the effect, and extends output by
  the largest SoX-ng-style drain length, including the delay-line tail, the
  final drain frame, and the extra samples required by linear and quadratic
  interpolation.
- Coverage includes typed processor tests, command and chain integration
  tests, L4 finite-output property coverage, parser fuzz seeds, layered
  coverage metadata, and standalone SoX-ng golden cases for linear mono chorus
  and multi-stage stereo chorus.

### Feature 6.5.6: `flanger`

Status: implemented.

Implementation notes:

- Added a public `Flanger` effect with SoX-ng's delay, depth, regeneration,
  width, speed, wave, phase, and interpolation parameters. Defaults match
  SoX-ng: 0 ms base delay, 2 ms depth, 0% regeneration, 71% width, 0.5 Hz
  speed, sine wave, 25% channel phase shift, and linear interpolation.
- Wired `flanger [-n|-l|-q] [-s|-t] [delay [depth [regen [width [speed
  [shape [phase [interp]]]]]]]]` through the effect registry, typed command
  parser, effect-chain execution, effects-file diagnostics, CLI positional
  chain path, and parser fuzz corpus.
- Processing uses independent per-channel delay lines, delayed-signal feedback,
  SoX-ng-style dry/wet mix balancing, full-scale clipping, and preserves input
  length. It intentionally does not emit a delayed tail; users who want the
  final feedback/delay residue should pad before applying `flanger`.
- Coverage includes analytical/unit, integration, L4 finite-output property,
  parser fuzz, layered coverage metadata, and standalone SoX-ng golden cases
  for linear mono and triangle/no-interpolation stereo flanger commands.

### Feature 6.5.7: `phaser`

Status: implemented.

Implementation notes:

- Added a public `Phaser` effect with SoX-ng's gain-in, gain-out, delay,
  regeneration, speed, sine/triangle modulation, and none/linear/quadratic
  interpolation parameters. Defaults match SoX-ng: gain-in 0.4, gain-out 0.74,
  3 ms delay, 0.4 regeneration, 0.5 Hz speed, sine wave, and no interpolation.
- Wired `phaser [-n|-l|-q] [-s|-t] [gain-in [gain-out [delay [regen [speed
  [-s|-t]]]]]]` through the effect registry, typed command parser, effect-chain
  execution, effects-file diagnostics, CLI positional chain path, and parser
  fuzz corpus.
- Processing uses channel-local feedback delay lines, clips after output gain,
  preserves input length, and intentionally does not emit a delayed tail.
- Coverage includes analytical/unit, integration, L4 finite-output property,
  parser fuzz, layered coverage metadata, and standalone SoX-ng golden cases
  for linear mono and triangle/no-interpolation stereo phaser commands.

### Feature 6.5.8: `reverb`

Status: implemented.

Implementation notes:

- Added a public `Reverb` effect with SoX-ng's `-w` wet-only mode and
  reverberance, HF damping, room scale, stereo depth, pre-delay, and wet-gain
  parameters.
- Wired `reverb [-w] [reverberance [HF-damping [room-scale [stereo-depth
  [pre-delay [wet-gain]]]]]]` through the effect registry, typed command
  parser, effect-chain execution, effects-file diagnostics, CLI positional
  chain path, and parser fuzz corpus.
- Processing uses SoX-ng's Freeverb-derived comb/all-pass delay network,
  preserves input length, keeps the command-line output channel shape stable,
  and does not drain the delayed wet tail after the input ends.
- Coverage includes unit/integration tests, L4 finite-output coverage, parser
  fuzz, layered coverage metadata, and standalone SoX-ng golden cases for
  mono wet-only and stereo dry-plus-wet reverb commands.

These effects are stateful. Each feature must document latency, tail behavior,
flush behavior, and whether chunked output is exact or tolerance-based.

## Milestone 6.6: sample-rate and time-domain effects

### Feature 6.6.1: `downsample`

Status: implemented.

Implementation notes:

- Added a public `Downsample` effect with SoX-ng's optional integer factor,
  default factor `2`, and supported range `1..=16384`.
- Wired `downsample [factor]` through the effect registry, typed command
  parser, effect-chain execution, effects-file diagnostics, CLI positional
  chain path, and parser fuzz corpus.
- Processing is simple decimation: keep frames `0, factor, 2 * factor, ...`,
  preserve channel count, update sample-rate metadata to `input_rate / factor`,
  and perform no anti-alias filtering.
- Coverage includes unit/integration tests, L4 finite-output and factor-one
  identity properties, parser fuzz, layered coverage metadata, and standalone
  SoX-ng golden cases for default mono and explicit-factor stereo commands.
- The current API is whole-buffer. It has no delay tail or flush phase, and
  chunk-exact streaming would require exposing the SoX-ng-style carry state.

### Feature 6.6.2: `upsample`

Status: implemented.

Implementation notes:

- Added a public `Upsample` effect with SoX-ng's optional integer factor,
  default factor `2`, and supported range `1..=256`.
- Wired `upsample [factor]` through the effect registry, typed command parser,
  effect-chain execution, effects-file diagnostics, CLI positional chain path,
  and parser fuzz corpus.
- Processing is simple zero stuffing: preserve each input frame, insert
  `factor - 1` zero frames after it, preserve channel count, update sample-rate
  metadata to `input_rate * factor`, and perform no reconstruction filtering.
- Coverage includes unit/integration tests, L4 finite-output and factor-one
  identity properties, parser fuzz, layered coverage metadata, and standalone
  SoX-ng golden cases for default mono and explicit-factor stereo commands.
- The current API is whole-buffer. It has no delay tail or flush phase, and
  chunk-exact streaming would require exposing the SoX-ng-style insertion phase
  across chunk boundaries.

### Feature 6.6.3: `speed`

Status: implemented.

Implementation notes:

- Added a public `Speed` effect with SoX-ng's required `factor[c]` argument:
  positive ratio values are accepted directly, while a trailing `c` converts
  cents through `2^(cents / 1200)`.
- Wired `speed factor[c]` through the effect registry, typed command parser,
  effect-chain execution, effects-file diagnostics, CLI positional chain path,
  and parser fuzz corpus.
- Processing preserves decoded samples, frame count, channel count, and sample
  format while updating sample-rate metadata to
  `round(input_rate * factor)`. It performs no interpolation or anti-alias
  filtering; later `rate` features own resampling quality.
- Coverage includes unit/integration tests, L4 factor-one identity and
  finite-output coverage, parser fuzz, layered coverage metadata, and
  standalone SoX-ng golden cases for mono speed-up and stereo slow-down
  commands with explicit output sample rates.
- The current API is whole-buffer. It has no delay tail or flush phase, and
  chunk-exact streaming is metadata-only once callers expose chunk-level
  sample-rate state.

### Feature 6.6.4: `rate` specification and scaffolding

Status: implemented.

Implementation notes:

- Added a public `Rate` effect scaffold and `rate frequency` command parser.
  The target frequency accepts integer hertz and `k`/`K` kilohertz shorthand,
  then renders as canonical integer hertz.
- Processing uses deterministic scalar linear resampling, preserves channel
  count and sample format, updates sample-rate metadata to the explicit target,
  and rounds output frames as `round(input_frames * target / source)`.
- SoX-ng quality flags and override options are deliberately rejected for now;
  Feature 6.6.5 owns quick/low-quality mode behavior, Feature 6.6.6 owns
  high-quality modes, and Feature 6.6.7 owns override options.
- Coverage includes analytical and integration tests for parsing, rendering,
  chain execution, identity-rate behavior, output length overflow, L4
  matching-rate identity, parser fuzz seed coverage, and narrow standalone
  SoX-ng goldens for mono silence conversion and stereo identity-rate command
  behavior.

### Feature 6.6.5: `rate` quick and low-quality modes

Status: implemented.

Implementation notes:

- `Rate` now records a `RateQuality` family in its typed API, with default,
  SoX-ng `-q` quick, and SoX-ng `-l` low-quality modes.
- The command parser accepts `rate -q frequency`, `rate -l frequency`, and the
  equivalent `rate -Q 0 frequency` / `rate -Q 1 frequency` forms, rendering
  them canonically as `-q` or `-l`.
- Quick and low-quality modes currently use the same deterministic scalar
  linear resampling scaffold as the default mode. Feature 6.6.6 later added
  the higher-quality selectors; override options remain scheduled for Feature
  6.6.7.
- Coverage includes parser/rendering tests, chain integration tests, fuzz seeds,
  and narrow standalone SoX-ng golden cases for quick mono silence conversion
  and low-quality stereo identity behavior.

### Feature 6.6.6: `rate` high-quality modes

Status: implemented.

Implementation notes:

- `RateQuality` now covers SoX-ng quality levels 2 through 7: `-m` medium,
  `-g` generic, `-h` high, `-e` extreme, `-v` very high, and `-u` ultra.
- The command parser accepts both shorthand options and equivalent `-Q 2`
  through `-Q 7` forms, rendering canonical shorthand tokens.
- These modes currently use the same deterministic scalar linear resampling
  scaffold as the earlier default, quick, and low-quality modes. Override
  options remain scheduled for Feature 6.6.7.
- Coverage includes typed constructor and parser/rendering tests, chain
  boundary tests, fuzz seeds, and narrow standalone SoX-ng golden cases for
  medium-rate conversion on silence and ultra identity behavior.

### Feature 6.6.7: `rate` override options

Status: implemented.

Implementation notes:

- `Rate` now records a `RateOptions` struct covering SoX-ng control flags
  `-i`, `-c`, `-f`, `-n`, and `-t`, plus high-quality override flags for
  phase, bandwidth, aliasing, and precision.
- The command parser accepts phase options `-M`, `-I`, `-L`, and `-p`,
  bandwidth options `-s`, `-b`, and `-B`, aliasing controls `-A` and `-a`,
  and precision controls `-d` and `-R`, while preserving SoX-ng's rule that
  high-quality overrides require medium quality or higher when an explicit
  quality selector is present.
- Override options currently feed the existing deterministic scalar linear
  resampling scaffold as typed metadata. Full polyphase/FIR quality behavior
  remains future implementation work behind the same public model.
- Coverage includes typed validation, parser/rendering, chain-boundary tests,
  fuzz seed coverage, and narrow standalone SoX-ng golden cases for high
  quality override parsing on mono silence and generic custom override parsing
  on stereo identity-rate input.

### Feature 6.6.8: `stretch`

Status: implemented.

Implementation notes:

- Added a public `Stretch` processor with SoX-ng's `factor`, `window`, `fade`,
  `shift`, and `fading` options, including linear, sqrt, half-cosine, and
  quarter-cosine cross-fade families.
- Processing mirrors SoX-ng's channel-local window/shift state machine,
  preserves sample-rate metadata, clips the mixed output to full scale, and
  treats factor `1` as a null effect.
- Wired `stretch` through the effect registry, typed command parser/rendering,
  effect-chain execution, effects-file diagnostics, CLI positional chain path,
  and fuzz smoke corpus.
- Coverage includes unit/integration/property tests, standalone mono/stereo
  SoX-ng golden cases, and an L0-L7 layered coverage matrix entry. SIMD is
  documented as N/A for this scalar stateful window-overlap implementation.

### Feature 6.6.9: `tempo` core

Status: implemented.

Implementation notes:

- Added a public `Tempo` processor for the default SoX-ng `tempo factor`
  profile, including the required factor range `0.1..=100`, sample-rate
  preservation, pitch-preserving scalar overlap-search processing, and
  factor-one identity behavior.
- Wired `tempo factor` through the effect registry, typed command parser,
  effect-chain execution, effects-file diagnostics, CLI positional chain path,
  and fuzz smoke corpus.
- Coverage includes unit/integration/property tests, standalone mono/stereo
  SoX-ng golden cases, and an L0-L7 layered coverage matrix entry. SIMD is
  documented as N/A for this scalar stateful overlap-search implementation.
- Tuning flags `-q`, `-m`, `-s`, and `-l`, plus explicit
  `segment/search/overlap` arguments, remain scheduled for Feature 6.6.10.

### Feature 6.6.10: `tempo` tuning options

Status: implemented.

Implementation notes:

- Extended the public `Tempo` model with SoX-ng's quick hierarchical search
  flag, `default`/`music`/`speech`/`linear` tuning profiles, and explicit
  millisecond `segment`, `search`, and `overlap` parameters.
- Wired `tempo [-q] [-m|-s|-l] factor [segment [search [overlap]]]` through
  the typed command parser, canonical rendering, effect-chain token grouping,
  CLI positional chain path, parser fuzz corpus, and standalone golden
  manifests.
- Processing derives unspecified tuning values with SoX-ng's profile formulas
  before building the scalar overlap-search state. `-l` uses a zero default
  search span, explicit overlap is capped to half the segment length, and
  output frame count remains `round(input_frames / factor)`.
- Coverage includes parser/rendering tests, chain integration tests, profile
  and explicit-tuning processor coverage, parser fuzz seeds, L0-L7 coverage
  metadata, and standalone SoX-ng golden cases for quick speech and explicit
  linear tuning.

### Feature 6.6.11: `pitch`

Status: implemented.

Implementation notes:

- Added a public `Pitch` effect that accepts shifts in cents, validates
  SoX-ng's cents-derived factor range, preserves duration by reusing the scalar
  `Tempo` overlap-search core with the inverse pitch factor, and updates output
  sample-rate metadata to the pitch factor.
- Wired `pitch [-q] shift [segment [search [overlap]]]` through effect
  registry resolution, typed command parsing/rendering, effect-chain execution,
  CLI positional chain path, and parser fuzz coverage. The SoX-ng `-m`, `-s`,
  and `-l` tempo profiles remain `tempo`-only because SoX-ng `pitch` exposes
  only the quick-search flag plus explicit timing values.
- Coverage includes unit and integration tests for parsing, metadata changes,
  stereo preservation, zero-cent identity, invalid shift/tuning rejection,
  parser fuzz seeds, L0-L7 coverage metadata, and standalone SoX-ng golden
  cases for octave-up mono and quick tuned octave-down stereo pitch shifting.

### Feature 6.6.12: `bend`

Status: implemented.

Implementation notes:

- Added a public `Bend` effect with `BendSegment`, `BendPosition`,
  frame-rate `-f`, and oversampling `-o` support. Positions accept SoX-ng-style
  seconds and `s`-suffixed frame counts, including `+` relative segment ends.
- Processing preserves duration and sample-rate metadata through a channel-local
  scalar phase-vocoder path backed by `rustfft`. SIMD is documented as N/A
  because the current transform is stateful STFT/overlap processing rather than
  a data-parallel sample kernel.
- Wired `bend [-f frame-rate] [-o oversample] {start(+),cents,end(+)}` through
  the typed command parser, effect registry, effect-chain dispatch, CLI
  positional chain path, parser fuzz corpus, L0-L7 coverage metadata, and
  standalone SoX-ng golden manifest.
- Coverage includes parser/rendering tests, chain integration tests, typed
  validation tests, zero-duration identity, finite-output property coverage,
  invalid option/position checks, and standalone mono/stereo zero-duration
  command-shape golden cases.

### Feature 6.6.13: `splice`

Status: implemented.

Implementation notes:

- Added a public `Splice` effect with `SpliceFade`, `SplicePoint`,
  `SplicePosition`, and `SpliceAmount` typed APIs. The command surface supports
  SoX-ng `splice [-h|-t|-q] {position[,excess[,leeway]]}` forms, including
  frame-count `s` suffixes, seconds, colon time parsing, and canonical
  rendering.
- Processing applies whole-buffer scalar cross-faded joins with SoX-ng-style
  excess/leeway resolution: explicit excess and leeway are doubled, the default
  excess/leeway are 5 ms per side, overlap has a 16-frame floor and 8-frame
  alignment, and `-q` uses zero default search.
- Wired `splice` through the effect registry, typed command parser, effect-chain
  dispatch, CLI positional-chain path, parser fuzz corpus, L0-L7 coverage
  metadata, and standalone SoX-ng golden manifest.
- Coverage includes parser/rendering tests, chain integration tests, typed
  validation tests, output-length and finite-sample assertions, property
  finite-output coverage, invalid point checks, and mono/stereo standalone
  golden cases.

## Milestone 6.7: dynamics, silence, and noise effects

### Feature 6.7.1: `compand` parser and transfer function

### Feature 6.7.2: `compand` processor

### Feature 6.7.3: `mcompand`

### Feature 6.7.4: `loudness`

### Feature 6.7.5: `silence`

### Feature 6.7.6: `vad` core

### Feature 6.7.7: `vad` advanced options

### Feature 6.7.8: `noiseprof`

### Feature 6.7.9: `noisered`

Dynamics and noise effects require deterministic state handling and careful
golden tolerances.

## Milestone 6.8: FIR, analysis, generation, and dither effects

### Feature 6.8.1: `fir` coefficient input

### Feature 6.8.2: `fir` streaming processor

### Feature 6.8.3: `firfit`

### Feature 6.8.4: `hilbert`

### Feature 6.8.5: `sinc` low-pass and high-pass

### Feature 6.8.6: `sinc` band-pass and band-reject

### Feature 6.8.7: `dither` TPDF and sloped TPDF

Implement deterministic dither primitives and an explicit `dither` effect before
any automatic insertion policy.

Acceptance tests:

- explicit seed/config policy is documented;
- tests do not rely on implicit randomness;
- SoX-ng dither golden tests use dither-specific invocation rules, not `-D`;
- silence and near-zero behavior is tested.

### Feature 6.8.8: automatic dither insertion policy

Moved here from the old Feature 5.5.4.

Implement only after Feature 6.8.7 exists.

Acceptance tests:

- automatic insertion rules are explicit and testable;
- library APIs expose dither insertion as an output-boundary policy, not hidden
  behavior;
- disabling dither is possible in tests;
- SoX-ng comparison tests record when SoX-ng auto-inserted `dither`;
- deterministic seed or repeatability behavior is documented;
- the policy composes correctly with guard, norm, sample-rate conversion, and
  channel conversion.

### Feature 6.8.9: `dither` noise shaping

Add noise-shaping modes after the base dither behavior and automatic insertion
policy are stable.

### Feature 6.8.10: `stat`

### Feature 6.8.11: `stats`

### Feature 6.8.12: `synth` basic waveforms

### Feature 6.8.13: `synth` noise, sweep, and combine modes

Analysis and generation effects must define deterministic output and metadata
behavior before CLI integration.

## Milestone 6.9: specialized and integration effects

### Feature 6.9.1: `dolbyb` feasibility and spec

Record whether a safe, testable implementation path exists. If blocked, the CLI
diagnostic must be stable and actionable.

### Feature 6.9.2: `dolbyb` implementation

Implement only if Feature 6.9.1 records a safe implementation path.

### Feature 6.9.3: `dop`

### Feature 6.9.4: `earwax`

### Feature 6.9.5: `ladspa` host or stable block

If blocked, CLI diagnostics must be stable and actionable.

### Feature 6.9.6: `sdm` feasibility and spec

Record whether a safe, testable implementation path exists. If blocked, the CLI
diagnostic must be stable and actionable.

### Feature 6.9.7: `sdm` implementation

Implement only if Feature 6.9.6 records a safe implementation path.
