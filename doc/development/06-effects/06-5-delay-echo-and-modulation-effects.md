---
kind: historical-roadmap
status: superseded
superseded_by:
  - ../effects.md
  - ../dsp.md
  - ../oracles.md
  - ../testing.md
---

## Milestone 6.5: delay, echo, and modulation effects

This milestone record is retained for implemented effect and primitive history
and coverage evidence. New effect, DSP, or oracle planning belongs in the flat
short documents under `../effects/`, `../dsp/`, and `../oracles/`.

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
