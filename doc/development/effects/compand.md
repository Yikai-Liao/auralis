---
kind: effect
effect: "compand"
status: planned
owner: "crates/auralis-effects/src/compand.rs"
graph_op: "compand"
cli_example: "auralis render input.wav -o output.wav --fx compand,curve=-60:-60,-20:-10,0:-3,attack=5ms,release=100ms"
family: "dynamics"
oracle_policy: "sox-ng"
oracle: "doc/development/oracles/dynamics.md"
---

# Compand

## One-Line Description

`compand` applies a SoX-compatible dynamics transfer curve with attack and
release smoothing.

## Reference Target

- Primary reference: SoX-ng semantics for compatibility.
- Secondary reference: Faust compressors for a future modern compressor effect.
- SoX-ng role: oracle for this compatibility effect.
- Why SoX-ng: the product value of `compand` is its curve syntax and historical
  compatibility.

## Parameters

| Param | Type | Default | Validation | Meaning |
| --- | --- | --- | --- | --- |
| `curve` | curve | required | sorted valid points | input/output dB curve |
| `attack` | `Milliseconds` | required | `> 0ms` | envelope attack |
| `release` | `Milliseconds` | required | `> 0ms` | envelope release |
| `gain` | `Decibels` | `0dB` | finite | makeup gain |
| `delay` | `Milliseconds` | `0ms` | `>= 0ms` | optional lookahead delay |

## Signal Model

Same sample rate and channel count. Length may include delay compensation only
if explicitly documented.

## Mathematics

```text
level[n] = envelope(abs(x[n]), attack, release)
gain_db[n] = curve(level_db[n]) - level_db[n] + makeup_gain
out[n] = delay_line(x[n], delay) * db_to_linear(gain_db[n])
```

## Pseudocode

```text
parse and validate curve
for sample in samples:
  env = update_envelope(abs(sample))
  gain = curve_gain(env)
  output = delayed_sample * gain
```

## Implementation Notes

Keep SoX compatibility here. A modern compressor/limiter should be a separate
effect with a Faust-derived oracle.

## Performance Notes

Envelope state is scalar and order-dependent. SIMD is not first-pass priority.

## Validation And Diagnostics

Reject invalid curve points, attack/release <= 0, non-finite gain, and invalid
delay.

## Tests

SoX-ng golden fixtures, curve interpolation tests, attack/release behavior,
delay behavior, and validation failures.

## Benchmarks

Benchmark long mono/stereo buffers for typical speech/music curves.

## Done When

SoX-compatible behavior is explicit and separate from future modern dynamics
effects.
