---
kind: effect
effect: "chorus"
status: planned
owner: "crates/auralis-effects/src/chorus.rs"
graph_op: "chorus"
cli_example: "auralis render input.wav -o output.wav --fx chorus,voices=3,depth=8ms,rate=0.4Hz,mix=50%"
family: "delay/modulation"
oracle_policy: "modern-reference"
oracle: "doc/development/oracles/modulation-delay.md"
---

# Chorus

## One-Line Description

`chorus` thickens audio by mixing several slightly delayed and modulated copies
with the dry signal.

## Reference Target

- Primary reference: DaisySP Chorus and Auralis fractional-delay design.
- Secondary reference: Faust delay-modulation structures.
- SoX-ng role: parameter compatibility reference only.
- Why not SoX-ng: SoX-ng's no-interpolation default is a historical speed
  tradeoff, not Auralis' default quality target.

## Parameters

| Param | Type | Default | Validation | Meaning |
| --- | --- | --- | --- | --- |
| `voices` | `u8` | `3` | `1..=16` | modulated delay voices |
| `depth` | `Milliseconds` | `8ms` | `> 0ms` | modulation depth |
| `rate` | `Hertz` | `0.4Hz` | `> 0Hz` | LFO rate |
| `feedback` | `f32` | `0` | `-0.95..=0.95` | feedback amount |
| `mix` | `Percent` | `50%` | `0%..=100%` | wet amount |

## Signal Model

One audio input, one audio output, same sample rate and channel count. Length
may include a documented tail only if enabled by a future parameter.

## Mathematics

```text
delay_v[n] = base_delay_v + depth * lfo_v[n]
wet[n] = sum_v gain_v * fractional_delay(input, delay_v[n])
out[n] = (1 - mix) * x[n] + mix * wet[n]
```

## Pseudocode

```text
validate params
for channel in channels:
  initialize voice delay lines and phases
  for n in frames:
    wet = sum delayed voices
    output[n] = dry*x[n] + wet_gain*wet
    advance phases
```

## Implementation Notes

Use `fractional-delay` once its contract is stable. Keep SoX-compatible parsing
separate from the default graph op.

## Performance Notes

Start with scalar fractional delay. Optimize voice state layout before SIMD.

## Validation And Diagnostics

Reject zero voices, invalid rate/depth, feedback outside range, and impossible
max delay.

## Tests

Dry mix identity, finite impulse response, stereo phase behavior, validation,
and property-level comparison against modulation-delay fixtures.

## Benchmarks

Benchmark 30 s and 300 s stereo buffers with 3 and 8 voices. Use fresh output
dirs and confirmation reruns.

## Done When

Fractional-delay behavior is documented, SoX-ng is not the quality oracle, and
fixtures cover modulation family behavior.
