---
kind: dsp-primitive
primitive: "fixed-direct-fir"
status: planned
owner: "crates/auralis-dsp/src/fir.rs"
extracted_from:
  - "fir"
  - "earwax"
future_users:
  - "loudness"
  - "sinc"
  - "hilbert"
---

# Fixed Direct FIR

## One-Line Description

`fixed_direct_fir` evaluates short fixed FIR filters with a stable boundary and
accumulation contract.

## Why This Is A Primitive

FIR, earwax, loudness, sinc, and Hilbert-related code all need direct
convolution. A shared primitive gives tests and SIMD one stable owner.

## Mathematical Contract

```text
out[n] = sum_{k=0..K-1} coeff[k] * input[n - k]
```

Boundary samples use the caller-selected state or padding policy; the primitive
must make that policy explicit.

## API Sketch

```rust
pub fn fir_direct(input: &[f32], coeffs: &[f32], output: &mut [f32]);
pub fn fir_11_direct(input: &[f32], coeffs: &[f32; 11], output: &mut [f32]);
```

- allocation behavior: no allocation for direct evaluation.
- panic/error behavior: debug assert or typed error on invalid lengths.
- state object: optional FIR state owned by a separate stateful wrapper.
- backend selection: scalar first, SIMD through `fixed-direct-fir`.

## Pseudocode

```text
for n in output frames:
  acc = 0
  for k in taps:
    acc += coeff[k] * sample_at(input, n - k, boundary_policy)
  output[n] = acc
```

## Callers

- FIR effect direct processor;
- earwax fixed filter;
- loudness filter support;
- sinc and Hilbert helpers where direct FIR is appropriate.

## What Stays Outside

Coefficient design, file-backed coefficient loading, effect CLI syntax,
resampling policy, and output file I/O.

## Numerical Notes

Accumulation precision, FMA tolerance, tap order, and boundary behavior must
match scalar tests.

## Performance Notes

Specialize common short tap counts such as 11 taps only after scalar fixtures
are stable. Avoid allocation inside sample loops.

## Tests

Impulse response, all-zero/all-one fixtures, 11-tap fixture, boundary policy,
vector-width lengths, and caller-level earwax/FIR regressions.

## Benchmarks

Benchmark 64K and 10M sample buffers for 11 taps and representative fixed tap
counts. Confirm SIMD wins with a second `--skip-build` run.

## Done When

Direct FIR behavior is shared, boundary policy is explicit, and at least one
FIR-heavy caller uses the primitive.
