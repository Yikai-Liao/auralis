---
kind: historical-roadmap
status: superseded
superseded_by:
  - ../effects.md
  - ../oracles.md
  - ../testing.md
---

## Milestone 6.2: volume, level, and simple modulation effects

This milestone record is retained for implemented effect history and coverage
evidence. New effect or oracle planning belongs in the flat short documents
under `../effects/` and `../oracles/`.

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
