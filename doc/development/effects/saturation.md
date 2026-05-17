---
kind: effect
effect: "saturation"
status: planned
owner: "crates/auralis-effects/src/saturation.rs"
graph_op: "saturation"
cli_example: "auralis render input.wav -o output.wav --fx saturation,curve=tanh,drive=40%,mix=100%"
family: "saturation/distortion"
oracle_policy: "modern-reference"
oracle: "doc/development/oracles/saturation-distortion.md"
---

# Saturation

## One-Line Description

`saturation` applies a named soft-clipping curve to add harmonic density while
limiting peaks.

## Reference Target

- Primary reference: Auralis-defined tanh/atan/softclip curves.
- Secondary reference: MusicDSP and Airwindows listening references.
- SoX-ng role: baseline only.
- Why not SoX-ng: saturation should be defined by Auralis curve semantics, not
  by one legacy waveshaper.

## Parameters

| Param | Type | Default | Validation | Meaning |
| --- | --- | --- | --- | --- |
| `curve` | enum | `tanh` | `tanh`, `atan`, `softclip` | transfer family |
| `drive` | `Percent` | `40%` | `0%..=100%` | input gain/intensity |
| `mix` | `Percent` | `100%` | `0%..=100%` | wet amount |

## Signal Model

Same length, sample rate, and channel count. No whole-buffer analysis.

## Mathematics

```text
pre = gain(drive) * x
tanh:     wet = tanh(pre) / tanh(gain(drive))
atan:     wet = atan(k*pre) / atan(k)
softclip: wet = piecewise_softclip(pre)
out = (1 - mix) * x + mix * wet
```

## Pseudocode

```text
validate curve and params
for sample in samples:
  wet = apply_selected_curve(sample, drive)
  output = dry*sample + mix*wet
```

## Implementation Notes

Curve formulas must be documented before adding variants. Airwindows is a
listening reference, not an automatic code source.

## Performance Notes

Vector math or polynomial approximations need explicit error policy before SIMD
work.

## Validation And Diagnostics

Reject unknown curve and invalid percentages.

## Tests

Curve identity at low drive, finite output, bounded output where promised,
odd/even symmetry according to curve, and spectral summaries.

## Benchmarks

Benchmark default and high-drive curves separately, especially `tanh` cost.

## Done When

Each curve has a formula, tests, and benchmark coverage or an explicit deferral.
