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
---

# AXPY

## One-Line Description

`axpy` computes `y = a*x + y` over same-length `f32` slices without allocation.

## Why This Is A Primitive

Mixing, remix routing, wet/dry blending, and gain staging all need the same
scale-and-accumulate loop. A shared primitive gives SIMD one stable target.

## Mathematical Contract

```text
for i in 0..N:
  y_i' = a * x_i + y_i
```

Invariants: same lengths, empty input is a no-op, no clipping or output guard
policy, NaN and infinity follow normal floating-point propagation.

## API Sketch

```rust
pub fn axpy(a: f32, x: &[f32], y: &mut [f32]);
```

- allocation behavior: no allocation.
- panic/error behavior: debug assert or typed error on length mismatch.
- state object: none.
- backend selection: scalar first, SIMD through `axpy-scale-mul-clamp`.

## Pseudocode

```text
assert same length
for i in 0..len:
  y[i] = a * x[i] + y[i]
```

## Callers

- `mix.sum`: apply edge gain and accumulate into output.
- `remix`: route source channel into target channel.
- wet/dry helpers: blend wet signal into dry output.

## What Stays Outside

Graph op names, CLI syntax, SoX-ng compatibility, clipping, normalization,
output guard policy, and file I/O.

## Numerical Notes

Use explicit multiply then add unless Auralis accepts `mul_add` rounding
differences and documents tolerance for scalar-vs-SIMD tests.

## Performance Notes

The loop is memory-bandwidth sensitive. Prefer contiguous planar slices, avoid
temporary buffers, and test SIMD tails around vector width.

## Tests

Empty, one-sample, odd-length, vector-boundary, seeded random finite data,
NaN/infinity propagation, and caller-level mix/remix regressions.

## Benchmarks

Run 1K, 64K, and 10M sample buffers plus one caller-level mix benchmark.
Confirm SIMD wins with a second `--skip-build` run before treating them as
final.

## Done When

Formula-level tests exist, at least one effect uses the primitive, and SIMD is
implemented or explicitly deferred.
