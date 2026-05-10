## Milestone 6.8: FIR, analysis, generation, and dither effects

### Feature 6.8.1: `fir` coefficient input

Status: implemented.

Implementation notes:

- Added public `Fir`, `FirCoefficientSource`, and `FirCoefficients` types for
  SoX-ng-style coefficient acquisition without registering `fir` as an
  executable effect yet.
- `Fir::parse_sox_args` mirrors SoX-ng's command split: no arguments read
  coefficients from standard input, one argument is a coefficient-file path
  even when it looks numeric, and two or more arguments are parsed as inline
  finite coefficients.
- `FirCoefficients::parse_text` parses coefficient-file text with
  whitespace-separated finite numbers and `#` comments, preserving empty
  coefficient files as empty lists because SoX-ng treats them as a null effect.
- Coverage includes unit and doc tests for command-shape parsing, canonical
  rendering, coefficient text comments, empty files, malformed inline input,
  and non-finite coefficient rejection. This feature intentionally does not add
  registry, chain execution, fuzz seeds, or SoX-ng golden rows; Feature 6.8.2
  owns the streaming FIR processor and executable command surface.

### Feature 6.8.2: `fir` streaming processor

Status: implemented.

Implementation notes:

- Registered `fir [coefs-file | coef <coef>]` as an executable
  SoX-ng-style effect command with canonical rendering, effect-chain grouping,
  file-backed coefficient loading, CLI positional-chain support, diagnostics,
  fuzz seed coverage, and L0-L7 matrix metadata.
- Added deterministic scalar FIR processing over decoded planar `AudioBuffer`
  samples. Empty coefficient lists are null effects, inline coefficients and
  explicit coefficient files execute in library/CLI chains, and command-style
  stdin is rejected at processing time because library chain execution cannot
  safely read interactive stdin.
- The processor preserves input frame count and follows SoX-ng FIR alignment by
  dropping `(coefficient_count - 1) / 2` leading convolution samples before
  returning the length-preserving output.
- Coverage includes unit and integration tests for command parsing, chain
  grouping, inline and file-backed execution, per-channel filtering, null
  coefficients, chunked-state equivalence, mono/stereo standalone SoX-ng
  golden rows, and L7 command fuzz seeds. SIMD remains N/A for this scalar
  reference feature until a vectorized convolution backend is planned.

### Feature 6.8.3: `firfit`

Status: implemented.

Implementation notes:

- Registered `firfit [knots-file | <freq gain>]` as an executable
  SoX-ng-style effect command with canonical rendering, effect-chain grouping,
  CLI positional-chain support, diagnostics, and L7 fuzz seed coverage.
- Added public `FirFit`, `FirFitKnot`, and `FirFitKnotSource` APIs for stdin,
  file-backed, and inline frequency/gain knot sources. Knot text supports
  whitespace-separated pairs and `#` comments; inline/file data must contain
  finite gains and strictly increasing non-negative frequencies.
- Chain execution supports inline and explicit knot-file sources, rejects
  stdin at processing time, designs exact centered-impulse coefficients for
  flat responses, and uses a deterministic scalar log-frequency interpolation
  scaffold for non-flat fitted responses.
- Coverage includes parser/rendering tests, file-backed chain execution,
  flat-response identity tests, non-flat finite-output coverage, mono/stereo
  SoX-ng flat-response golden rows, and a layered coverage matrix row. SIMD
  remains N/A because this feature designs scalar FIR coefficients and then
  runs the scalar reference FIR processor.

### Feature 6.8.4: `hilbert`

Status: implemented.

Implementation notes:

- Registered `hilbert [-n taps]` as an executable SoX-ng-style effect command
  with parser/rendering, registry metadata, effect-chain grouping and
  dispatch, CLI positional-chain support, L7 fuzz seed coverage, and standalone
  SoX-ng golden rows.
- Added a public `Hilbert` processor that validates SoX-ng's odd tap-count
  range, derives default taps from the input sample rate using the 75 Hz cutoff
  heuristic, generates the Blackman-windowed Hilbert FIR coefficients, and
  delegates length-preserving scalar processing to the shared FIR executor.
- Coverage includes tap validation, default tap derivation, analytical
  coefficient checks, chain equivalence with typed processing, finite-output
  coverage, chunked FIR-state equivalence, and mono/stereo explicit-tap golden
  cases. SIMD remains N/A until a future vectorized FIR backend is planned.

### Feature 6.8.5: `sinc` low-pass and high-pass

Status: implemented.

Implementation notes:

- Registered `sinc [options] freq` and `sinc [options] -freq` as executable
  SoX-ng-style high-pass and low-pass FIR filters, with canonical parsing and
  rendering for attenuation, beta, transition bandwidth, explicit taps,
  auto-tap rounding, and low-pass delete-at-Nyquist metadata.
- Added a public `Sinc` processor that designs deterministic scalar
  Kaiser-windowed low-pass coefficients, inverts them for high-pass mode, and
  delegates length-preserving execution to the shared FIR processor.
- Coverage includes parser/rendering tests, chain grouping and execution,
  finite-output property coverage, chunked FIR-state equivalence, parser fuzz
  coverage, L0-L7 matrix metadata, and explicit-tap mono/stereo SoX-ng golden
  cases. SIMD remains N/A until a future vectorized FIR backend is planned.
- Band-pass and band-reject frequency ranges remain rejected and scheduled for
  Feature 6.8.6.

### Feature 6.8.6: `sinc` band-pass and band-reject

Status: implemented.

Implementation notes:

- Extended the public `SincBand` model with SoX-ng-style `low-high`
  band-pass and reversed `high-low` band-reject ranges.
- Band filters reuse the deterministic scalar Kaiser low-pass designer,
  centered coefficient composition, and length-preserving FIR executor from
  Feature 6.8.5.
- Wired band range parsing, canonical rendering, effect-chain grouping, parser
  fuzz coverage, L0-L7 coverage metadata, and standalone SoX-ng golden cases.
- Coverage includes parser/rendering tests, invalid equal-edge rejection,
  typed/chain processor equivalence, finite-output coverage, centered
  band-pass/band-reject complement checks, and mono/stereo explicit-tap
  goldens. SIMD remains N/A until a future vectorized FIR backend exists.

### Feature 6.8.7: `dither` TPDF and sloped TPDF

Status: implemented.

Implemented deterministic dither primitives and an explicit `dither` effect
before any automatic insertion policy.

- Added public `Dither`, `DitherMode`, and `DitherState` APIs covering plain
  TPDF, `-S` sloped TPDF, `-p bits` target precision, and explicit deterministic
  seed configuration.
- Registered `dither [-S] [-p precision]` through the effect registry, command
  parser/renderer, effect-chain dispatch, CLI positional chain path, and parser
  fuzz corpus.
- Added Rust parser/processor/chunk-state tests, finite-output property
  coverage, mono/stereo standalone SoX-ng golden cases, and L0-L7 layered
  coverage metadata.
- SoX-ng golden dither cases run with `-R` and an explicit `dither` command
  rather than relying on automatic `-D` behavior. Noise shaping is covered by
  Feature 6.8.9, while automatic on/off detection (`-a`) remains scheduled for
  a later dither feature.

Acceptance tests:

- explicit seed/config policy is documented;
- tests do not rely on implicit randomness;
- SoX-ng dither golden tests use dither-specific invocation rules, not `-D`;
- silence and near-zero behavior is tested.

### Feature 6.8.8: automatic dither insertion policy

Moved here from the old Feature 5.5.4.

Status: implemented.

Implemented explicit output-boundary dither insertion now that Feature 6.8.7
provides deterministic dither primitives.

Implementation notes:

- Added `OutputDitherPolicy`, `OutputDitherConfig`, and
  `Pipeline::with_output_dither_policy` / `Pipeline::with_output_dither` so
  library callers opt in explicitly instead of receiving hidden dither noise.
- Output dither runs after output sample-rate conversion, channel conversion,
  guard, and normalization, and before PCM16 encoding.
- The high-level library and CLI default to disabled dither. The CLI exposes
  opt-in deterministic TPDF dither as `--dither`; `--dither-seed` selects a
  repeatable seed and is rejected unless `--dither` is present.
- Golden coverage records `sox_ng_auto_dither = true` and runs SoX-ng without
  `-D` for the automatic-dither reference case.

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

Status: implemented.

Implementation notes:

- Extended the public `Dither` model with `DitherNoiseShape::Shibata`,
  including the SoX-ng `dither -s` shorthand and `dither -f shibata` command
  forms.
- Noise-shaped processing uses deterministic TPDF input plus stateful
  error-feedback shaping before the same target-precision quantization used by
  the base dither path. The typed seed policy is preserved, and chunked
  `DitherState` processing remains deterministic for shaped dither.
- Coverage includes parser/rendering tests, command and chain integration,
  finite/quantized output checks, chunk-state equivalence, parser fuzz corpus,
  L0-L7 matrix updates, and a standalone SoX-ng golden case for explicit
  `dither -s -p 8`. Additional named SoX-ng shaping filters and automatic
  on/off detection remain unimplemented.

### Feature 6.8.10: `stat`

Status: implemented.

Implementation notes:

- Added a public `Stat` analyzer and `StatReport` artifact model for
  SoX-ng-style sample statistics over decoded audio.
- Registered `stat [-s scale] [-rms] [-v] [-j]` across command parsing,
  rendering, registry resolution, effect-chain grouping and dispatch, CLI
  positional-chain execution, L7 fuzz seed coverage, standalone pass-through
  SoX-ng golden rows, and the L0-L7 coverage matrix.
- The analyzer computes statistics in frame-major stream order, matching
  SoX-ng's interleaved processing order while preserving Auralis' planar buffer
  layout. Chain execution passes audio through unchanged; callers can collect
  deterministic text, JSON, or volume-adjustment renderings through the typed
  report API.
- Coverage includes parser/rendering tests, chain pass-through execution,
  frame-major multichannel statistics, report rendering, invalid scale
  rejection, non-finite sample rejection, mono/stereo standalone goldens, and a
  fuzz corpus seed. `-freq`, `-a`, `-d`, `-e`, and `-h` remain explicit
  unsupported options pending dedicated spectrum/EBU artifact support.

### Feature 6.8.11: `stats`

Status: implemented.

Implementation notes:

- Added a public `Stats` analyzer and `StatsReport` artifact model for
  SoX-ng-style overall and per-channel sample statistics over decoded audio.
- Registered `stats [-b bits|-x bits|-s scale] [-w window-time] [-j]` across
  command parsing, rendering, registry resolution, effect-chain grouping and
  dispatch, CLI positional-chain execution, L7 fuzz seed coverage, standalone
  pass-through SoX-ng golden rows, and the L0-L7 coverage matrix.
- Chain execution passes audio through unchanged. The typed report API exposes
  deterministic text and JSON renderers with DC offset, min/max levels,
  peak/RMS dB levels, moving RMS peak/trough, crest/flat factors, peak counts,
  bit-depth estimates, sample count, length, and window metadata.
- Coverage includes parser/rendering tests, chain pass-through execution,
  overall/per-channel statistics, report rendering, invalid option-range
  rejection, non-finite sample rejection, mono/stereo standalone goldens, and a
  fuzz corpus seed. Chunk-exact streaming accumulation and output-artifact
  capture remain future API work because the current analyzer is whole-buffer.

### Feature 6.8.12: `synth` basic waveforms

Implemented in this branch.

- Added public `Synth`, `SynthChannel`, `SynthLength`, and `SynthWaveform`
  APIs for create-mode tonal waveform generation over decoded audio shapes.
- Supported deterministic SoX-ng-style `sine`, `square`, `sawtooth`,
  `triangle`, `trapezium`, and `exp` oscillators with optional length,
  frequency, offset, phase, and shape parameters.
- Registered `synth [-n] [length] waveform [frequency ...]` across the effect
  registry, typed command parser/renderer, effect-chain dispatch, CLI
  positional-chain path, fuzz corpus, standalone SoX-ng golden manifest, and
  L0-L7 layered coverage metadata.
- Coverage includes waveform formula tests, parser/rendering tests,
  chain-grouping and execution tests, finite-output property coverage,
  mono/stereo standalone golden cases, invalid parameter rejection, and stable
  unsupported diagnostics for noise, sweeps, and input-combine modes. SIMD is
  documented as N/A because this feature is scalar phase evaluation rather than
  a backend-dispatched sample transform.

### Feature 6.8.13: `synth` noise, sweep, and combine modes

Status: implemented.

Implementation notes:

- Extended the public `Synth` model with deterministic SoX-ng-style
  `whitenoise`, `tpdfnoise`, `pinknoise`, and `brownnoise` generation, typed
  frequency sweeps, and `create`, `mix`, `amod`, `fmod`, and `vdelay` combine
  modes.
- Wired noise aliases, `:`, `+`, `/`, and `-` sweep frequency tokens, and
  combine-mode parsing/rendering through the command parser, effect-chain
  grouping, CLI positional-chain path, fuzz corpus, standalone golden
  manifest, and L0-L7 coverage metadata.
- Processing uses the repeatable SoX-ng linear-congruential PRNG seed for
  command-style noise and preserves frame-major random draw order across
  channels while writing Auralis' planar buffer layout.
- Coverage includes deterministic PRNG assertions, sweep and combine parser
  tests, `fmod` and `vdelay` processing checks, finite-output property
  coverage, mono SoX-ng golden rows for whitenoise, linear sweep, and mix, and
  continued stereo basic-waveform golden coverage. SIMD remains N/A because
  synth is scalar oscillator/noise state generation rather than a
  backend-dispatched sample transform.
