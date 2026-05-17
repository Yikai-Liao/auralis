---
kind: effect
effect: "pitch"
status: planned
owner: "crates/auralis-effects/src/pitch.rs"
graph_op: "pitch"
cli_example: "auralis render input.wav -o output.wav --fx pitch,semitones=2"
family: "time/pitch"
oracle_policy: "modern-reference"
oracle: "doc/development/oracles/time-pitch.md"
---

# Pitch

## One-Line Description

`pitch` shifts perceived pitch while preserving duration as closely as the
selected backend allows.

## Reference Target

- Primary reference: Signalsmith Stretch.
- Quality reference: Rubber Band CLI as an external comparison.
- SoX-ng role: baseline and compatibility reference only.
- Why not SoX-ng: SoX-ng `pitch` shares the same historical WSOLA family as
  `tempo`, which should not define Auralis' default quality target.

## Parameters

| Param | Type | Default | Validation | Meaning |
| --- | --- | --- | --- | --- |
| `semitones` | `f64` | required | finite | pitch shift amount |
| `cents` | `f64` | `0` | finite | additional cents |
| `quality` | enum | `balanced` | `fast`, `balanced`, `high` | quality/cost tradeoff |

## Signal Model

- Inputs: one audio port.
- Outputs: one audio port named `audio`.
- Changes length: no, after documented compensation.
- Changes sample rate: no.
- Changes channels: no.
- Whole-buffer state: time/pitch backend analysis state.

## Mathematics

```text
ratio = 2 ^ ((semitones + cents/100) / 12)
y = pitch_shift_preserve_duration(x, ratio)
```

## Pseudocode

```text
validate pitch params
ratio = semitone_to_ratio(semitones, cents)
prepare backend
shift pitch with duration compensation
trim_or_pad to input frame count according to policy
```

## Implementation Notes

Keep direct resampling and duration compensation hidden behind the shared
time/pitch backend. Do not expose Rubber Band as a linked dependency.

## Performance Notes

Benchmark upward and downward shifts because analysis and resampling costs may
differ by backend strategy.

## Validation And Diagnostics

Reject non-finite semitone/cents values, unsupported quality, and output length
overflow.

## Tests

Sine frequency shift, fixed duration, silence, finite output, validation, and
metric comparison against time/pitch oracle fixtures.

## Benchmarks

Cases: `semitones=-2`, `2`, and `7`. Use fresh output dirs and `--skip-build`
confirmation before claiming a win.

## Done When

Pitch ratio math is explicit, duration policy is deterministic, and quality
fixtures are tied to the time/pitch oracle.
