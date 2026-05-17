---
kind: effect
effect: "overdrive"
status: planned
owner: "crates/auralis-effects/src/overdrive.rs"
graph_op: "overdrive"
cli_example: "auralis render input.wav -o output.wav --fx overdrive,drive=60%,tone=50%,mix=100%"
family: "saturation/distortion"
oracle_policy: "modern-reference"
oracle: "doc/development/oracles/saturation-distortion.md"
---

# Overdrive

## One-Line Description

`overdrive` applies a controlled nonlinear transfer curve to add harmonic
distortion.

## Reference Target

- Primary reference: DaisySP Overdrive for minimal structure.
- Secondary reference: MusicDSP waveshaper formulas.
- SoX-ng role: baseline only.
- Why not SoX-ng: overdrive is creative nonlinear DSP; Auralis should define
  its own curve rather than inherit an old tool's sound.

## Parameters

| Param | Type | Default | Validation | Meaning |
| --- | --- | --- | --- | --- |
| `drive` | `Percent` | `50%` | `0%..=100%` | input gain/curve intensity |
| `tone` | `Percent` | `50%` | `0%..=100%` | simple tone-shaping control |
| `mix` | `Percent` | `100%` | `0%..=100%` | wet amount |

## Signal Model

Same length, sample rate, and channel count. No whole-buffer analysis required.

## Mathematics

```text
pre = gain(drive) * x
wet = softclip(pre)
out = (1 - mix) * x + mix * tone_filter(wet, tone)
```

## Pseudocode

```text
validate params
for sample in samples:
  pre = input_gain * sample
  wet = transfer_curve(pre)
  output = dry*sample + mix*tone_filter(wet)
```

## Implementation Notes

Start with one documented transfer curve. Add named curve variants only after
tests and listening references exist.

## Performance Notes

Avoid expensive transcendental functions unless the curve requires them and
benchmarks justify the cost.

## Validation And Diagnostics

Reject invalid percentages and unknown curve/tone modes.

## Tests

Dry identity, monotonic curve where applicable, finite output, symmetry or
asymmetry according to curve, and reference spectral summaries.

## Benchmarks

Benchmark long mono/stereo buffers for default and high-drive settings.

## Done When

Transfer curve is explicit, tests prove finite bounded behavior, and SoX-ng is
not the default oracle.
