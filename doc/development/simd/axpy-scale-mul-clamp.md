---
kind: simd-kernel
kernel: "axpy-scale-mul-clamp"
status: planned
priority: "A2"
owner: "crates/auralis-simd/src/linear.rs"
scalar_reference: "linear_scalar"
simd_backend: "rten-simd"
callers:
  - "auralis-dsp axpy"
  - "mix"
  - "remix"
  - "wet/dry"
---

# AXPY Scale Multiply Clamp

## One-Line Description

Provide reusable SIMD kernels for `y += a*x`, `y = a*x`, `y = x*m`, and clamp.

## SIMD Justification

Each output element depends only on same-index input elements and scalar
parameters. Lanes are independent and tails can use scalar reference logic.

## Scalar Formula

```text
axpy:       y[i] = y[i] + a * x[i]
scale_into: y[i] = a * x[i]
mul_slice:  y[i] = x[i] * m[i]
clamp:      y[i] = min(max(x[i], lo), hi)
```

## SIMD Pseudocode

```text
for vector indexes:
  x_v = load(x)
  y_v = load(y) when needed
  m_v = load(m) when needed
  store(output, fused_op(x_v, y_v, m_v, scalar_params))
handle tail with scalar loop
```

## Numerical Contract

- Use the same finite/NaN behavior as scalar `f32` operations.
- `axpy` may use FMA only if scalar and SIMD tolerance accounts for it.
- Clamp preserves NaN behavior according to the chosen scalar implementation.
- Empty slices are no-ops.

## API Sketch

```rust
pub fn axpy(y: &mut [f32], a: f32, x: &[f32]);
pub fn scale_into(out: &mut [f32], a: f32, x: &[f32]);
pub fn mul_by_slice(out: &mut [f32], x: &[f32], m: &[f32]);
pub fn clamp_in_place(values: &mut [f32], lo: f32, hi: f32);
```

## Tail And Fallback

- Length mismatches are caller or validation errors, not silently truncated.
- Tail lengths cover all vector-width boundaries.
- Unsupported targets fall back to scalar.

## Tests

- exact small fixtures;
- zero-length slices;
- vector-width boundary lengths;
- seeded random data;
- NaN and infinity policy fixtures;
- callers through mix/remix/wet-dry paths.

## Benchmarks

- Input sizes: 1K, 64K, 10M samples.
- Cases: each primitive alone and remix-like accumulate.
- Baseline: scalar `auralis-dsp` implementation.
- Confirmation: fresh output dir plus second `--skip-build` run.

## Done When

- Shared scalar references exist.
- All four primitives have forced scalar-vs-SIMD tests.
- Callers can use the shared backend-capable API.
