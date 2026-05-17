---
kind: dsp-primitive
primitive: "mul-by-slice"
status: planned
owner: "crates/auralis-dsp/src/linear.rs"
extracted_from:
  - "tremolo"
future_users:
  - "modulation"
  - "envelope_apply"
---

# Multiply By Slice

## One-Line Description

`mul_by_slice` applies a same-length modulation or envelope slice to audio
samples.

## Why This Is A Primitive

Tremolo, envelope application, and future modulation effects all need the same
elementwise multiply after their control signal is generated.

## Mathematical Contract

```text
for i in 0..N:
  out_i = x_i * m_i
```

Invariants: same lengths, empty input is a no-op, no control-signal generation
inside the primitive.

## API Sketch

```rust
pub fn mul_by_slice(input: &[f32], multiplier: &[f32], output: &mut [f32]);
pub fn mul_by_slice_in_place(samples: &mut [f32], multiplier: &[f32]);
```

- allocation behavior: no allocation.
- panic/error behavior: debug assert or typed error on length mismatch.
- state object: none.
- backend selection: scalar first, SIMD through `axpy-scale-mul-clamp`.

## Pseudocode

```text
assert same length
for i in 0..len:
  output[i] = input[i] * multiplier[i]
```

## Callers

- tremolo: apply precomputed LFO modulator;
- envelope apply: apply gain envelope;
- future modulation effects after control generation.

## What Stays Outside

LFO generation, envelope generation, parameter validation, phase state, and
graph or CLI syntax.

## Numerical Notes

NaN and infinity propagate as scalar multiplication. No clamping or finite
repair happens here.

## Performance Notes

Use planar contiguous slices. Keep control generation separate so SIMD can
target the pure multiply loop.

## Tests

Empty, one-sample, vector-boundary lengths, all-ones multiplier, all-zero
multiplier, seeded random data, and caller-level tremolo regression.

## Benchmarks

Run 1K, 64K, and 10M sample buffers plus a tremolo apply benchmark. Confirm
SIMD evidence with a second `--skip-build` run.

## Done When

The primitive has scalar tests, tremolo can use it, and SIMD parity is tested
or explicitly deferred.
