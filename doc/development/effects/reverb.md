---
kind: effect
effect: "reverb"
status: planned
owner: "crates/auralis-effects/src/reverb.rs"
graph_op: "reverb"
cli_example: "auralis render input.wav -o output.wav --fx reverb,room=50%,decay=1.8s,mix=25%"
family: "reverb"
oracle_policy: "modern-reference"
oracle: "doc/development/oracles/reverb.md"
---

# Reverb

## One-Line Description

`reverb` adds simulated acoustic space using a documented feedback delay
network or related reverberator structure.

## Reference Target

- Primary reference: Faust reverbs, especially FDN/Moorer/Schroeder designs.
- Secondary reference: Freeverb only as a small or compatibility baseline.
- SoX-ng role: SoX-compatible Freeverb baseline only.
- Why not SoX-ng: SoX-ng's Freeverb-style implementation is useful but should
  not be the only default Auralis reverb target.

## Parameters

| Param | Type | Default | Validation | Meaning |
| --- | --- | --- | --- | --- |
| `room` | `Percent` | `50%` | `0%..=100%` | room size proxy |
| `decay` | `Seconds` | `1.8s` | `> 0s` | decay time |
| `damping` | `Percent` | `50%` | `0%..=100%` | high-frequency damping |
| `width` | `Percent` | `100%` | `0%..=100%` | stereo decorrelation |
| `mix` | `Percent` | `25%` | `0%..=100%` | wet amount |

## Signal Model

One audio input, stereo output by default unless input policy says otherwise.
Length and tail policy must be explicit.

## Mathematics

```text
v[n] = FDN_or_comb_allpass_network(x[n], decay, damping)
out[n] = (1 - mix) * x[n] + mix * decorrelate(v[n], width)
```

## Pseudocode

```text
validate params
initialize delay network from sample_rate and room
for n in input plus optional tail:
  wet = process_reverb_network(input_or_zero[n])
  output[n] = dry*input_or_zero[n] + mix*wet
```

## Implementation Notes

Choose one default algorithm before implementation. Keep Freeverb compatibility
as an explicit mode or separate plan.

## Performance Notes

State layout and delay-buffer locality matter more than SIMD first pass.

## Validation And Diagnostics

Reject invalid decay, mix, damping, width, and impossible delay sizes.

## Tests

Silence, impulse decay, finite bounded output, stereo decorrelation fixtures,
and reference metric comparison against Faust reverb family.

## Benchmarks

Benchmark 30 s and 300 s stereo buffers plus impulse-tail cases.

## Done When

Default algorithm is named, tail policy is deterministic, and Freeverb compat is
not confused with the default target.
