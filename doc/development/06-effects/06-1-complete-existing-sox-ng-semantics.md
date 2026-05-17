---
kind: historical-roadmap
status: superseded
superseded_by:
  - ../effects.md
  - ../oracles.md
  - ../testing.md
---

## Milestone 6.1: complete existing SoX-ng semantics

This milestone record is retained for implemented effect history and coverage
evidence. New effect or oracle planning belongs in the flat short documents
under `../effects/` and `../oracles/`.

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
