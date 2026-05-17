---
kind: effect
effect: "mcompand"
status: planned
owner: "crates/auralis-effects/src/mcompand.rs"
graph_op: "mcompand"
cli_example: "auralis render input.wav -o output.wav --fx mcompand,bands=low:...,high:..."
family: "dynamics"
oracle_policy: "sox-ng"
oracle: "doc/development/oracles/dynamics.md"
---

# MCompand

## One-Line Description

`mcompand` applies SoX-compatible companding independently across multiple
frequency bands.

## Reference Target

- Primary reference: SoX-ng semantics for compatibility.
- Secondary reference: Faust compressors for future modern multiband dynamics.
- SoX-ng role: oracle for this compatibility effect.
- Why SoX-ng: `mcompand` is valuable primarily as a legacy multiband compander
  surface.

## Parameters

| Param | Type | Default | Validation | Meaning |
| --- | --- | --- | --- | --- |
| `bands` | list | required | non-empty valid bands | per-band compand configs |
| `crossovers` | list | required | sorted, within Nyquist | band split points |
| `gain` | `Decibels` | `0dB` | finite | global makeup gain |

## Signal Model

One audio input and output, same length/sample rate/channel count, with
band-split and recombine latency documented.

## Mathematics

```text
band_b[n] = crossover_b(x[n])
processed_b[n] = compand_b(band_b[n])
out[n] = sum_b processed_b[n] * global_gain
```

## Pseudocode

```text
validate crossovers and band configs
split input into bands
for band in bands:
  apply compand curve and envelope
sum processed bands
apply global gain
```

## Implementation Notes

Reuse `compand` curve/envelope only after its contract is isolated from command
syntax. Keep modern multiband compression separate.

## Performance Notes

Crossover filters and envelope state dominate. SIMD is not first-pass priority.

## Validation And Diagnostics

Reject empty bands, unsorted crossovers, crossover >= Nyquist, invalid nested
compand configs, and non-finite gain.

## Tests

SoX-ng golden fixtures, crossover validation, per-band curve tests, recombine
finite output, and validation failures.

## Benchmarks

Benchmark typical 2-band and 4-band configurations on long stereo buffers.

## Done When

SoX-compatible multiband behavior is documented and isolated from future modern
dynamics work.
