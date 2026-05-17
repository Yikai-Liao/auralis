---
kind: dsp-primitive
primitive: "axpy"
status: planned
owner: "crates/auralis-dsp/src/axpy.rs"
extracted_from:
  - "mix"
  - "remix"
future_users:
  - "mix"
  - "remix"
  - "wet_dry"
  - "gain_staging"
---

# AXPY DSP Primitive Example

## One-Line Description

`axpy` computes `y = a*x + y` over same-length `f32` slices without allocation.

## Why This Is A Primitive

Mixing, remix routing, wet/dry blending, and gain staging all need the same
scale-and-accumulate loop. Keeping it as a primitive prevents duplicate
effect-local kernels and gives SIMD one stable target.

## Mathematical Contract

```text
for i in 0..N:
  y_i' = a * x_i + y_i
```

Invariants:

- `x.len() == y.len()`
- empty input is a no-op
- no clipping, normalization, or headroom policy
- NaN and infinity follow normal floating-point propagation

## API Sketch

```rust
pub fn axpy(a: f32, x: &[f32], y: &mut [f32]);
```

- allocation behavior: no allocation
- panic/error behavior: debug assert on length mismatch in phase 1
- state object: none
- backend selection: scalar first, SIMD wrapper after scalar tests land

## Pseudocode

```text
assert same length
for i in 0..len:
  y[i] = a * x[i] + y[i]
```

## Callers

- `mix.sum`: accumulates source buffers into one output.
- `remix`: accumulates source channels into target channels.
- wet/dry helpers: accumulates scaled wet signal into dry output.

## What Stays Outside

- graph op names and params
- CLI parsing
- clipping and output guard policy
- file I/O

## Numerical Notes

Use explicit multiply then add unless Auralis accepts scalar/SIMD `mul_add`
rounding differences. If `mul_add` is adopted, document the tolerance in caller
tests.

## Performance Notes

- Hot loop is memory-bandwidth sensitive.
- Prefer planar contiguous slices.
- Do not allocate temporary buffers.
- SIMD tail handling must be tested at lengths around vector width.

## Tests

- empty slice
- one sample
- odd length
- seeded random finite data
- NaN/infinity propagation
- caller-level regression through `mix` or `remix`

## Benchmarks

- input sizes: 1K, 64K, and 10M samples
- caller-level benchmark: `mix.sum` over stereo buffers
- primitive-level benchmark: scalar vs SIMD once SIMD exists
- confirmation rule: second skip-build run before claiming a win

## Done When

- The primitive has formula-level tests.
- At least one effect uses it.
- SIMD applicability is implemented or explicitly deferred with a reason.
