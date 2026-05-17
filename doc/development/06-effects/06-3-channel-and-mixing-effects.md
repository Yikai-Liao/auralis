---
kind: historical-roadmap
status: superseded
superseded_by:
  - ../effects.md
  - ../oracles.md
  - ../testing.md
---

## Milestone 6.3: channel and mixing effects

This milestone record is retained for implemented effect history and coverage
evidence. New effect or oracle planning belongs in the flat short documents
under `../effects/` and `../oracles/`.

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
