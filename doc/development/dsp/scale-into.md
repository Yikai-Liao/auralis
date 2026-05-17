---
kind: dsp-primitive
primitive: "scale-into"
status: planned
owner: "crates/auralis-dsp/src/linear.rs"
extracted_from:
  - "gain"
  - "normalize"
future_users:
  - "remix"
  - "wet_dry"
---

# Scale Into

## One-Line Description

`scale_into` writes `out = a*x` over same-length `f32` slices without
allocation.

## Why This Is A Primitive

Gain-style transforms, normalization apply passes, and buffer preparation paths
all need a pure scale kernel with no effect policy attached.

## Mathematical Contract

```text
for i in 0..N:
  out_i = a * x_i
```

Invariants: same lengths, empty input is a no-op, no clipping, and no finite
validation.

## API Sketch

```rust
pub fn scale_into(a: f32, input: &[f32], output: &mut [f32]);
pub fn scale_in_place(a: f32, samples: &mut [f32]);
```

- allocation behavior: no allocation.
- panic/error behavior: debug assert or typed error on length mismatch.
- state object: none.
- backend selection: scalar first, SIMD through `axpy-scale-mul-clamp`.

## Pseudocode

```text
assert same length
for i in 0..len:
  output[i] = a * input[i]
```

## Callers

- gain and volume effects for pure scale modes;
- normalize apply pass after peak/RMS analysis;
- wet/dry helpers before accumulation.

## What Stays Outside

Parameter parsing, dB conversion, output guard policy, normalization analysis,
and file I/O.

## Numerical Notes

Scalar and SIMD multiplication must use the same `f32` contract. NaN and
infinity propagate.

## Performance Notes

Prefer in-place operation when aliasing is explicit and safe. Avoid allocation
and keep the generic output path separate from effect-level validation.

## Tests

Empty, one-sample, alias-safe in-place, vector-boundary lengths, seeded random
data, NaN/infinity propagation, and caller-level gain/normalize regressions.

## Benchmarks

Run 1K, 64K, and 10M sample buffers. Add caller-level normalize apply timing
once normalize uses the primitive.

## Done When

Scalar tests pass, callers own effect policy outside the primitive, and SIMD
status is implemented or explicitly deferred.
