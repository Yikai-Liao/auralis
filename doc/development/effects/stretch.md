---
kind: effect
effect: "stretch"
status: planned
owner: "crates/auralis-effects/src/stretch.rs"
graph_op: "stretch"
cli_example: "auralis render input.wav -o output.wav --fx stretch,factor=1.25"
family: "time/pitch"
oracle_policy: "modern-reference"
oracle: "doc/development/oracles/time-pitch.md"
---

# Stretch

## One-Line Description

`stretch` changes audio duration without intending to change pitch.

## Reference Target

- Primary reference: Signalsmith Stretch for normal musical time stretching.
- Secondary reference: PaulStretch / PaulXStretch for a separate future extreme
  stretch mode.
- Quality reference: Rubber Band CLI for CI comparison where licensing allows
  external reference use.
- SoX-ng role: legacy baseline only.
- Why not SoX-ng: SoX-ng `stretch` is a short-window cross-fade algorithm with
  weak quality and output-length quirks, so it should not define Auralis'
  default algorithm.

## Parameters

| Param | Type | Default | Validation | Meaning |
| --- | --- | --- | --- | --- |
| `factor` | `f64` | required | `> 0` | output duration multiplier |
| `mode` | enum | `standard` | `standard` only at first | algorithm family |
| `transient` | enum | `auto` | `auto`, `smooth`, `preserve` | transient handling preference |
| `formant` | enum | `off` | `off`, future `preserve` | formant handling |
| `quality` | enum | `balanced` | `fast`, `balanced`, `high` | analysis/window tradeoff |

## Signal Model

- Inputs: one audio port, mono or multi-channel.
- Outputs: one audio port named `audio`.
- Changes length: yes, approximately `input_frames * factor`.
- Changes sample rate: no.
- Changes channels: no.
- Whole-buffer state: analysis windows, phase/history state, and explicit
  output length selection.

## Mathematics

The target family is phase-consistent overlap/add, not SoX-style simple
cross-fade:

```text
X_m[k] = STFT(x[n], analysis_hop, window)
Y_l[k] = stretch_phase_update(X_m[k], phase_state, factor)
y[n] = ISTFT(Y_l[k], synthesis_hop, window)
```

For standard mode:

```text
analysis_pos = m * analysis_hop
synthesis_pos = round(analysis_pos * factor)
target_frames = round(input_frames * factor)
```

The exact phase update and transient policy should follow the chosen
Signalsmith-derived design note before implementation.

## Pseudocode

```text
validate factor and mode
choose window, analysis_hop, synthesis_hop from quality and sample_rate
initialize per-channel stretch state
for analysis_frame in overlapping_windows(input):
  spectrum = analyze_window(analysis_frame)
  adjusted = update_phase_and_transients(spectrum, state, factor)
  overlap_add(adjusted, output_accumulator)
trim_or_pad output to target_frames according to documented policy
```

## Implementation Notes

- Runtime type: `Stretch`.
- Params type: `StretchParams`.
- DSP helpers: shared STFT/window helpers may belong in `auralis-dsp` if also
  used by `tempo`, `pitch`, `bend`, or `noisered`.
- Edge params: none.
- Reports: optional analysis summary only after core behavior is stable.
- Keep extreme stretch out of this op until the separate oracle policy is
  written.

## Performance Notes

- FFT planning and window allocation must happen outside the frame loop.
- Multi-channel processing may parallelize by channel only if output ordering
  remains deterministic.
- Avoid allocating a new spectrum buffer per frame.
- Benchmark 0.75x, 1.25x, and 2.0x on speech and music fixtures.

## Validation And Diagnostics

- missing `factor`.
- `factor <= 0`.
- unsupported `mode`.
- incompatible `formant` value before formant preservation exists.
- output length overflow for huge inputs or factors.

## Tests

- `factor=1` preserves duration and remains close to input under the selected
  algorithm tolerance.
- Output frame count matches the documented target-frame rule.
- Silence remains finite silence.
- Sine fixtures preserve pitch within tolerance for moderate factors.
- Compare quality summaries against Signalsmith-derived fixtures and optional
  Rubber Band references, not SoX-ng sample equality.

## Benchmarks

- Cases: `factor=0.75`, `factor=1.25`, `factor=2.0`.
- Input shape: 48 kHz mono/stereo, 30 s speech and music.
- Metric: whole-buffer render time and peak allocation.
- Baseline: current Auralis scalar/window implementation if present.
- Confirmation: rerun with `--skip-build` before claiming a win.

## Done When

- Standard stretch has a documented algorithm target.
- Output length policy is deterministic.
- Tests cover pitch stability, duration, validation, and finite output.
- Extreme stretch is either separate or explicitly not implemented.
