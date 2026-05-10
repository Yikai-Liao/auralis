## Milestone 6.7: dynamics, silence, and noise effects

### Feature 6.7.1: `compand` parser and transfer function

Status: implemented.

Implementation notes:

- Added public `Compand`, `CompandAttackDecay`, `CompandTransfer`, and
  `CompandTransferPoint` typed APIs for the SoX-ng argument shape
  `attack,decay{,attack,decay} [soft-knee-dB:]in-dB1[,out-dB1]{,in-dB2,out-dB2}
  [gain [initial-volume-dB [delay]]]`.
- The typed parser validates finite non-negative attack/decay and delay
  values, finite post gain, initial volume at or below 0 dBFS, transfer levels
  at or below 0 dBFS, `-inf` transfer values, and strictly increasing transfer
  input levels.
- The transfer-function implementation mirrors SoX-ng's dB-space gain table:
  optional soft-knee parsing, the 0.01 dB minimum effective knee, automatic
  0,0 endpoint insertion, colinear point joining, quadratic knee segments, and
  post-gain application are covered by analytical unit tests.
- This feature intentionally does not register `compand` as an executable
  effect command and does not add L2 golden rows yet; Feature 6.7.2 owns the
  stateful envelope follower, delay handling, effect registry integration,
  CLI/chain dispatch, L0-L7 matrix row, fuzz seed, and SoX-ng golden coverage.

### Feature 6.7.2: `compand` processor

Status: implemented.

Implementation notes:

- Registered executable `compand` support for the SoX-ng command shape
  `attack,decay{,attack,decay} [soft-knee-dB:]in-dB1[,out-dB1]{,in-dB2,out-dB2}
  [gain [initial-volume-dB [delay]]]`, reusing the typed parser and transfer
  model from Feature 6.7.1.
- Processing follows SoX-ng's envelope follower: one attack/decay pair creates
  a shared frame-peak envelope for multichannel input, while multiple pairs
  must match the input channel count and track channels independently.
- Optional delay is implemented as length-preserving look-ahead, where the
  current envelope gain is applied to delayed samples and the buffered samples
  are drained without extending total frame count.
- Wired `compand` through the effect registry, typed command renderer,
  effect-chain dispatch, CLI positional-chain path, parser fuzz corpus, L0-L7
  coverage metadata, and standalone SoX-ng golden manifest.
- Coverage includes parser/rendering tests, chain integration tests,
  shared-envelope stereo behavior, delay behavior, invalid channel-group
  rejection, finite-output property coverage, and mono/stereo standalone
  golden cases.

### Feature 6.7.3: `mcompand`

Status: implemented.

Implementation notes:

- Added public `MCompand` and `MCompandBand` APIs for SoX-ng's
  `mcompand quoted_compand_args {crossover_frequency quoted_compand_args}`
  command shape, reusing the `Compand` transfer and envelope model for each
  band.
- Processing splits decoded input with scalar Linkwitz-Riley-style crossover
  pairs, applies each band compander independently, sums bands back to the
  original channel/frame shape, and clips the summed output to normalized full
  scale.
- Crossover frequencies must be positive and strictly ascending, with `k`/`K`
  kilohertz shorthand supported by the command parser.
- Nonzero per-band compander delays are rejected because SoX-ng parses them but
  its current `mcompand` flow path fails at runtime when delay buffering is
  active.
- Wired `mcompand` through the effect registry, typed command parser,
  effect-chain dispatch, CLI positional-chain path, parser fuzz corpus,
  L0-L7 coverage metadata, and standalone single-band SoX-ng golden manifest.
- Coverage includes parser/rendering tests, chain integration tests,
  single-band equivalence with `compand`, multiband finite-output coverage,
  invalid shape/order/delay checks, property finite-output coverage, and
  mono/stereo standalone goldens for the SoX-ng-compatible single-band path.

### Feature 6.7.4: `loudness`

Status: implemented.

Implementation notes:

- Added public `Loudness` support for SoX-ng's `loudness [gain [reference
  [n]]]` command shape, including default `-10 dB` gain, `65 dB` reference,
  and `1023` half-length settings plus SoX-ng's documented ranges.
- Processing builds a deterministic scalar centered FIR filter from the ISO
  226 table, natural cubic interpolation, inverse FFT response generation, and
  Kaiser windowing. A zero gain is treated as an identity command.
- Wired `loudness` through the effect registry, typed command parser/renderer,
  effect-chain dispatch, CLI positional-chain path, parser fuzz corpus, L0-L7
  coverage metadata, and standalone SoX-ng golden manifest.
- Coverage includes parser/rendering tests, command and chain integration
  tests, identity behavior, finite-output property coverage, invalid range
  checks, and mono/stereo standalone golden cases. SIMD is documented as N/A
  because the current implementation is a generated FIR convolution rather
  than a backend-dispatched per-sample kernel.

### Feature 6.7.5: `silence`

Status: implemented.

Implementation notes:

- Added public `Silence`, `SilenceDuration`, `SilenceThreshold`, and
  `SilencePeriod` support for SoX-ng's `silence [-l] above-periods [duration
  threshold] [below-periods duration threshold]` command family.
- Durations support SoX-ng's silence-specific parsing rule where bare numbers
  and `s` suffixes are sample/frame counts, while decimal, colon, and `t`
  forms are seconds resolved using the input sample rate.
- Whole-buffer processing supports leading trim, trailing trim, `-l` retained
  silence, and negative `below-periods` restart behavior for middle-silence
  removal. The current threshold detector is deterministic over decoded `f32`
  samples and keeps the command/API boundary ready for a later streaming state
  implementation.
- Coverage includes parser/rendering tests, command and chain integration
  tests, leading/trailing/middle trim behavior, invalid option/threshold
  checks, property identity/finite-output coverage, fuzz seed coverage, and
  standalone mono/stereo SoX-ng golden cases for leading-trim command behavior.

### Feature 6.7.6: `vad` core

Status: implemented.

Implementation notes:

- Added a public typed `Vad` core that performs deterministic whole-buffer
  leading non-voice trimming. The core detects voice with a normalized
  full-scale frame threshold, requires a configurable number of voice frames,
  can retain a fixed pre-trigger span, and can tolerate short quiet gaps while
  searching for the trigger.
- The implementation is deliberately not registered as an executable
  `EffectCommand` yet. Feature 6.7.7 owns SoX-ng command parsing, advanced VAD
  option mapping, L2 golden rows, fuzz seeds, and the L0-L7 matrix row.
- Coverage includes typed integration tests for leading trim, empty no-voice
  output, pre-trigger retention, quiet-gap tolerance, multichannel detection,
  invalid threshold rejection, and L4 finite-output property coverage. SIMD is
  documented as N/A for this structural detector because it is a whole-buffer
  search rather than a data-parallel sample transform.

### Feature 6.7.7: `vad` advanced options

Status: implemented.

Implementation notes:

- Registered `vad` as an executable SoX-ng-style effect command with parser,
  canonical rendering, registry metadata, chain/CLI dispatch, effects-file
  diagnostics, and fuzz seed coverage.
- Added a validated `VadOptions` profile for SoX-ng's advanced VAD options:
  boot/noise timing, measurement controls, spectral and cepstral frequency
  windows, trigger timing/level, search/gap timing, and pre-trigger retention.
  The profile is resolved against the input sample rate at processing time and
  mapped onto the deterministic whole-buffer VAD core from Feature 6.7.6.
- Coverage includes command parser/rendering tests, chain execution, invalid
  option ranges, the existing typed detector behavior, L4 finite-output
  property coverage, mono/stereo SoX-ng golden rows for no-voice trimming, and
  a layered coverage matrix row. SIMD remains N/A because VAD is a structural
  detector/trim operation rather than a data-parallel sample transform.

### Feature 6.7.8: `noiseprof`

Status: implemented.

Scope:

- Adds a public `NoiseProf` analyzer and `NoiseProfile` artifact model for
  SoX-ng-style 2048-point FFT noise profile collection.
- Supports `noiseprof [profile-file(-)]` command parsing/rendering, registry
  resolution, effects-file and positional-chain grouping, and chain execution
  as an audio pass-through command.
- Renders channel-major profile text using SoX-ng's `Channel N: ...` format so
  Feature 6.7.9 can consume the same stable profile shape for `noisered`.
- Coverage includes parser/rendering tests, chain pass-through execution,
  deterministic silence and stereo profile tests, mono/stereo SoX-ng
  pass-through golden rows, L7 fuzz seed coverage, and a layered coverage
  matrix row. SIMD remains N/A because the implemented analyzer is a
  whole-window FFT/statistics pass rather than a backend-dispatched sample
  transform.

### Feature 6.7.9: `noisered`

Status: implemented.

Scope:

- Adds a public `NoiseRed` processor that consumes the `NoiseProfile` text
  shape from Feature 6.7.8 or loads a command-style profile path at processing
  time.
- Supports `noisered [profile-file(-) [amount(0.5)]]` command
  parsing/rendering, registry resolution, effects-file and positional-chain
  grouping, chain execution, and `0..=1` amount validation.
- Implements deterministic scalar FFT-domain noise gating with 2048-frame
  windows, 50% overlap, smoothing, and SoX-ng's half-window output latency
  shape. SIMD remains N/A because the processor is an FFT/stateful spectral
  reducer rather than a backend-dispatched sample transform.
- Coverage includes profile text parsing, parser/rendering tests, chain
  grouping, typed zero-profile processing, mono/stereo SoX-ng command-shape
  golden rows with generated zero profiles, L7 fuzz seed coverage, and a
  layered coverage matrix row.
