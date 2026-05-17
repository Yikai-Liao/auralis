---
kind: effect
effect: "flanger"
status: planned
owner: "crates/auralis-effects/src/flanger.rs"
graph_op: "flanger"
cli_example: "auralis render input.wav -o output.wav --fx flanger,delay=2ms,depth=2ms,rate=0.5Hz,feedback=0.3"
family: "delay/modulation"
oracle_policy: "modern-reference"
oracle: "doc/development/oracles/modulation-delay.md"
---

# Flanger

## One-Line Description

`flanger` mixes audio with a short modulated delay to create moving comb-filter
coloration.

## Reference Target

- Primary reference: Faust `flanger_mono` / `flanger_stereo`.
- Secondary reference: DaisySP Flanger.
- SoX-ng role: parameter compatibility reference only.
- Why not SoX-ng: Auralis should use modern fractional delay interpolation
  rather than inheriting SoX-ng's quality tradeoffs.

## Parameters

| Param | Type | Default | Validation | Meaning |
| --- | --- | --- | --- | --- |
| `delay` | `Milliseconds` | `2ms` | `>= 0ms` | base delay |
| `depth` | `Milliseconds` | `2ms` | `>= 0ms` | delay sweep depth |
| `rate` | `Hertz` | `0.5Hz` | `> 0Hz` | LFO rate |
| `feedback` | `f32` | `0.3` | `-0.95..=0.95` | feedback amount |
| `mix` | `Percent` | `50%` | `0%..=100%` | wet amount |

## Signal Model

One audio input and output, same length unless a future explicit tail policy is
added, same sample rate and channel count.

## Mathematics

```text
d[n] = delay + depth * (0.5 + 0.5*sin(phase[n]))
wet[n] = fractional_delay(x[n] + feedback*wet[n-1], d[n])
out[n] = (1 - mix) * x[n] + mix * wet[n]
```

## Pseudocode

```text
validate params
for channel in channels:
  reset delay state
  for n in frames:
    delay_now = modulated_delay(phase)
    wet = delay_line.push_read(input[n] + feedback*prev_wet, delay_now)
    output[n] = dry*input[n] + mix*wet
    prev_wet = wet
```

## Implementation Notes

Share `fractional-delay` with chorus. Keep feedback and wet/dry policy in the
effect.

## Performance Notes

Optimize delay-line layout before SIMD. Benchmark feedback and no-feedback
cases separately.

## Validation And Diagnostics

Reject invalid delay/depth/rate, feedback outside range, and sweep exceeding
allocated max delay.

## Tests

Dry mix identity, finite impulse response, feedback stability, validation, and
comb movement metric fixtures.

## Benchmarks

Benchmark 48 kHz stereo 30 s and 300 s buffers. Confirm performance evidence
with `--skip-build`.

## Done When

Flanger uses documented fractional delay, has stability tests, and treats
SoX-ng as compat rather than quality oracle.
