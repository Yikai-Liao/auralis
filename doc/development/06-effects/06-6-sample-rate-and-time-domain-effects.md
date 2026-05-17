---
kind: historical-roadmap
status: superseded
superseded_by:
  - ../effects.md
  - ../oracles.md
  - ../testing.md
---

## Milestone 6.6: sample-rate and time-domain effects

This milestone record is retained for implemented effect history and coverage
evidence. New effect or oracle planning belongs in the flat short documents
under `../effects/` and `../oracles/`.

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
