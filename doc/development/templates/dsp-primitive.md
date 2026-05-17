---
kind: dsp-primitive
primitive: "<primitive-name>"
status: planned
owner: "crates/auralis-dsp/src/<primitive>.rs"
extracted_from:
  - "<effect-or-module>"
future_users:
  - "<effect-or-module>"
---

# DSP Primitive Template

Use this when extracting reusable DSP math from effects. Put document metadata
in the YAML front matter above. Keep the filled file under 250 lines.

## One-Line Description

Example:

`axpy` computes `y = a*x + y` over same-length `f32` slices without allocation.

## Why This Is A Primitive

Explain why this should leave an effect module.

Example:

This loop is useful in several mixing and blending effects, has a stable numeric
contract, and can be tested and vectorized independently of any op params.

## Mathematical Contract

Write equations and invariants.

Example:

```text
for i in 0..N:
  y_i' = a * x_i + y_i
```

Invariants:

- `x.len() == y.len()`
- empty input is a no-op
- no clipping or headroom policy
- IEEE NaN/infinity behavior is preserved

## API Sketch

```rust
pub fn axpy(a: f32, x: &[f32], y: &mut [f32]);
```

State:

- allocation behavior:
- panic/error behavior:
- state object, if any:
- backend selection, if any:

Example:

- allocation behavior: no allocation
- panic/error behavior: debug assert or error on length mismatch, to be decided
- state object: none
- backend selection: scalar first, SIMD if kernel plan is accepted

## Pseudocode

```text
assert same length
for i in 0..len:
  y[i] = mul_add(a, x[i], y[i])
```

## Callers

List how effects use it without leaking effect semantics into the primitive.

Example:

- `mix.sum`: applies edge gain and accumulates into output
- `wet_dry`: blends dry and wet buffers
- `remix`: accumulates routed source channel into target channel

## What Stays Outside

Example:

- op names and graph params
- CLI syntax
- SoX-ng command compatibility
- clipping, normalization, and output guard policy
- file I/O

## Numerical Notes

- exactness or tolerance:
- rounding behavior:
- accumulator precision:
- finite/NaN/infinity policy:

Example:

Use scalar `mul_add` only if Auralis accepts the resulting rounding difference;
otherwise use explicit multiply then add and make SIMD match scalar tolerance.

## Performance Notes

Example:

- Hot loop is memory-bandwidth sensitive.
- Prefer contiguous planar slices.
- SIMD should handle tails without scalar bugs.
- Do not allocate temporary buffers.

## Tests

- empty slice
- one sample
- odd length
- length around SIMD width
- seeded random finite data
- NaN/infinity if public behavior is defined
- caller-level regression through at least one effect

## Benchmarks

- input sizes:
- caller-level benchmark:
- primitive-level benchmark:
- confirmation rule:

Example:

Run 1K, 64K, and 10M sample buffers. Confirm SIMD wins with a second
skip-build run before treating as final.

## Done When

- Primitive has a formula-level contract.
- Effects using it still own effect policy.
- Tests prove scalar behavior.
- SIMD applicability is implemented or explicitly rejected.
