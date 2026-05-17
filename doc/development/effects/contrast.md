---
kind: effect
effect: "contrast"
status: planned
owner: "crates/auralis-effects/src/contrast.rs"
graph_op: "contrast"
cli_example: "auralis render input.wav -o output.wav --fx contrast,amount=50%"
family: "saturation/distortion"
oracle_policy: "modern-reference"
oracle: "doc/development/oracles/saturation-distortion.md"
---

# Contrast

## One-Line Description

`contrast` applies a nonlinear curve that increases or decreases sample-level
contrast around zero.

## Reference Target

- Primary reference: Auralis-defined nonlinear contrast curve.
- Secondary reference: MusicDSP waveshaper formulas.
- SoX-ng role: baseline only.
- Why not SoX-ng: the default behavior should be a documented Auralis curve,
  not a copied legacy transfer function.

## Parameters

| Param | Type | Default | Validation | Meaning |
| --- | --- | --- | --- | --- |
| `amount` | `Percent` | `50%` | `0%..=100%` | curve intensity |
| `mix` | `Percent` | `100%` | `0%..=100%` | wet amount |

## Signal Model

Same length, sample rate, and channel count. No whole-buffer analysis.

## Mathematics

```text
wet = contrast_curve(x, amount)
out = (1 - mix) * x + mix * wet
```

The chosen curve must define odd/even symmetry, bounds, and derivative behavior
near zero.

## Pseudocode

```text
validate amount and mix
for sample in samples:
  wet = contrast_curve(sample, amount)
  output = dry*sample + mix*wet
```

## Implementation Notes

Do not add SIMD until curve approximation and error tolerance are documented.

## Performance Notes

Benchmark scalar curve cost and consider polynomial approximations only with
explicit spectral/error tests.

## Validation And Diagnostics

Reject invalid percentages and unknown future curve names.

## Tests

Identity at `amount=0%`, finite output, curve bounds, symmetry, seeded random
samples, and spectral summaries.

## Benchmarks

Benchmark 30 s and 300 s stereo buffers for default and high amount.

## Done When

The contrast curve is documented mathematically and tests lock finite/bounded
behavior.
