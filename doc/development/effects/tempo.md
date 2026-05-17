---
kind: effect
effect: "tempo"
status: planned
owner: "crates/auralis-effects/src/tempo.rs"
graph_op: "tempo"
cli_example: "auralis render input.wav -o output.wav --fx tempo,factor=1.10"
family: "time/pitch"
oracle_policy: "modern-reference"
oracle: "doc/development/oracles/time-pitch.md"
---

# Tempo

## One-Line Description

`tempo` changes playback speed while preserving pitch as closely as the selected
time-stretch backend allows.

## Reference Target

- Primary reference: Signalsmith Stretch.
- Quality reference: Rubber Band CLI as an external comparison.
- SoX-ng role: baseline and compatibility reference only.
- Why not SoX-ng: SoX-ng WSOLA behavior should not lock Auralis' default
  quality or future phase/transient policy.

## Parameters

| Param | Type | Default | Validation | Meaning |
| --- | --- | --- | --- | --- |
| `factor` | `f64` | required | `> 0` | tempo multiplier |
| `quality` | enum | `balanced` | `fast`, `balanced`, `high` | quality/cost tradeoff |
| `transient` | enum | `auto` | `auto`, `smooth`, `preserve` | transient policy |

## Signal Model

- Inputs: one audio port.
- Outputs: one audio port named `audio`.
- Changes length: yes, approximately `input_frames / factor`.
- Changes sample rate: no.
- Changes channels: no.
- Whole-buffer state: analysis windows and phase/history state.

## Mathematics

```text
stretch_factor = 1 / factor
target_frames = round(input_frames * stretch_factor)
y = time_stretch(x, stretch_factor, quality, transient)
```

## Pseudocode

```text
validate factor
stretch_factor = 1 / factor
prepare time_pitch_backend(quality, transient)
render whole buffer through backend
trim_or_pad to documented target_frames
```

## Implementation Notes

Share the standard time/pitch backend with `stretch`, `pitch`, and `bend` where
possible. Keep SoX-compatible parsing outside this default graph op.

## Performance Notes

Reuse FFT/window plans and scratch buffers. Benchmark moderate factors on speech
and music before tuning quality defaults.

## Validation And Diagnostics

Reject missing factor, `factor <= 0`, unsupported quality, and target length
overflow.

## Tests

Duration rule, silence, sine pitch stability, speech/music metric fixtures, and
validation failures.

## Benchmarks

Cases: `factor=0.9`, `1.1`, and `1.5` on 48 kHz mono/stereo fixtures. Use fresh
benchmark output directories and confirm wins with `--skip-build`.

## Done When

Tempo shares the time/pitch oracle, output length is deterministic, and tests
cover pitch stability plus validation.
