---
kind: simd-kernel
kernel: "fixed-direct-fir"
status: planned
priority: "A7"
owner: "crates/auralis-simd/src/fir.rs"
scalar_reference: "fixed_fir_scalar"
simd_backend: "rten-simd"
callers:
  - "fir"
  - "earwax"
  - "loudness"
  - "sinc/hilbert support"
---

# Fixed Direct FIR

## One-Line Description

Accelerate fixed/direct FIR kernels, especially short filters such as 11-tap
FIR.

## SIMD Justification

Output samples are independent once each lane has access to the required input
window. Short fixed tap counts can use unrolled vector multiply-accumulate.

## Scalar Formula

```text
out[n] = sum_{k=0..taps-1} coeff[k] * input[n - k]
```

Boundary handling follows the caller's documented padding or state policy.

## SIMD Pseudocode

```text
for vector output positions:
  acc = 0
  for k in taps:
    x_v = load(input shifted by k)
    acc += splat(coeff[k]) * x_v
  store(output, acc)
handle prefix/suffix/tail with scalar reference
```

## Numerical Contract

- Tap order and boundary policy must match scalar exactly.
- FMA use must be reflected in tolerance or disabled for exact parity.
- Accumulation uses `f32` or `f64` according to the scalar contract.
- NaN/infinity propagate according to scalar arithmetic.

## API Sketch

```rust
pub fn fir_11_direct(input: &[f32], coeffs: &[f32; 11], output: &mut [f32]);
pub fn fir_direct(input: &[f32], coeffs: &[f32], output: &mut [f32]);
```

## Tail And Fallback

- Prefix samples requiring unavailable history use scalar boundary logic.
- Tail after vector chunks uses scalar logic.
- Unsupported tap counts may use generic scalar until a specialized path exists.
- Unsupported targets fall back to scalar.

## Tests

- impulse response equals coefficients under documented boundary policy;
- all-zero and all-one fixtures;
- 11-tap golden fixture;
- vector-width boundary lengths;
- caller-level tests for earwax or FIR effect;
- forced scalar-vs-SIMD differential tests.

## Benchmarks

- Input sizes: 64K and 10M samples.
- Tap counts: 11 first, then representative small fixed counts.
- Baseline: scalar direct FIR.
- Confirmation: fresh output dir plus second `--skip-build` run.

## Done When

- 11-tap specialized path is tested.
- Boundary behavior matches scalar.
- At least one FIR-heavy caller can use the backend-capable API.
