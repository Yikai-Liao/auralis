---
kind: dsp-primitive
primitive: "clamp"
status: planned
owner: "crates/auralis-dsp/src/linear.rs"
extracted_from:
  - "output policies"
future_users:
  - "remix"
  - "nonlinear effects"
---

# Clamp

## One-Line Description

`clamp` limits samples to a numeric range without applying normalization or
dither policy.

## Why This Is A Primitive

Output guards, remix policy, and some nonlinear effects need the same bounded
sample operation, but the decision to clamp belongs to callers.

## Mathematical Contract

```text
for i in 0..N:
  y_i = min(max(x_i, lo), hi)
```

Invariants: `lo <= hi`, empty input is a no-op, no finite validation unless the
caller requests it separately.

## API Sketch

```rust
pub fn clamp_in_place(samples: &mut [f32], lo: f32, hi: f32);
pub fn clamp_into(input: &[f32], output: &mut [f32], lo: f32, hi: f32);
```

- allocation behavior: no allocation.
- panic/error behavior: debug assert or typed error when `lo > hi`.
- state object: none.
- backend selection: scalar first, SIMD through `axpy-scale-mul-clamp`.

## Pseudocode

```text
assert lo <= hi
for i in 0..len:
  output[i] = min(max(input[i], lo), hi)
```

## Callers

- output policy before integer encoding;
- remix when a caller chooses bounded output;
- nonlinear effects that define bounded transfer curves.

## What Stays Outside

Whether clamping is appropriate, dither, integer quantization, normalization,
and warning/report generation.

## Numerical Notes

NaN behavior must be explicit: either preserve scalar `min/max` behavior or use
a documented finite-repair policy outside this primitive.

## Performance Notes

The kernel is memory-bandwidth sensitive and maps directly to SIMD min/max.
Avoid allocating temporary output unless caller needs out-of-place operation.

## Tests

Empty, one-sample, in-range, below-range, above-range, `lo == hi`, NaN policy,
vector-boundary lengths, and output-policy caller regression.

## Benchmarks

Run 1K, 64K, and 10M sample buffers. Confirm SIMD wins with a second
`--skip-build` run.

## Done When

NaN policy is explicit, scalar tests pass, and at least one caller uses the
primitive without leaking output policy into it.
