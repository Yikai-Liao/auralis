---
kind: simd-kernel
kernel: "tremolo-modulation"
status: planned
priority: "A3"
owner: "crates/auralis-simd/src/tremolo.rs"
scalar_reference: "tremolo_apply_scalar"
simd_backend: "rten-simd"
callers:
  - "tremolo"
---

# Tremolo Modulation

## One-Line Description

Apply a precomputed tremolo modulator slice to each audio channel.

## SIMD Justification

LFO generation may remain scalar, but once the modulation slice exists each
sample multiply is independent.

## Scalar Formula

```text
mod[n] = 1 - depth * lfo_shape(phase[n])
out[c,n] = input[c,n] * mod[n]
```

## SIMD Pseudocode

```text
for channel in channels:
  for vector in chunks:
    x_v = load(input[channel])
    m_v = load(modulator)
    store(output[channel], x_v * m_v)
  handle tail with scalar loop
```

## Numerical Contract

- Modulator values are produced by the scalar tremolo reference.
- SIMD apply must match scalar multiplication within `f32` tolerance.
- NaN and infinity propagate as scalar multiplication does.
- Empty channels are no-ops.

## API Sketch

```rust
pub fn apply_tremolo_modulator(out: &mut [f32], input: &[f32], modulator: &[f32]);
pub fn apply_tremolo_modulator_in_place(samples: &mut [f32], modulator: &[f32]);
```

## Tail And Fallback

- `input` and `modulator` lengths must match.
- Tail uses scalar multiplication.
- Unsupported targets fall back to scalar.

## Tests

- depth `0%` returns input;
- depth `100%` follows scalar modulator;
- vector-width boundary lengths;
- mono and stereo caller tests;
- seeded random input with deterministic modulator.

## Benchmarks

- Input sizes: 48 kHz stereo, 30 s and 300 s.
- Baseline: scalar tremolo apply loop.
- Metric: modulation apply time separate from LFO generation where possible.
- Confirmation: fresh output dir plus second `--skip-build` run.

## Done When

- LFO generation and SIMD apply boundaries are explicit.
- Scalar-vs-SIMD apply tests pass.
- Tremolo caller uses the shared apply kernel.
