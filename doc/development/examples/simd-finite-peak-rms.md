---
kind: simd-kernel
kernel: "finite_peak_rms"
status: planned
priority: "A1"
owner: "crates/auralis-simd/src/finite_peak_rms.rs"
scalar_reference: "finite_peak_rms_scalar"
simd_backend: "rten-simd"
callers:
  - "stats"
  - "normalize"
  - "write_validation"
---

# Finite Peak RMS SIMD Kernel Example

## One-Line Description

Compute finite sample count, absolute peak, and RMS sum of squares over an
`f32` slice.

## SIMD Justification

Each sample contributes independently to finite count, peak, and sum of
squares. Vector lanes can accumulate partial values and reduce once per chunk.

## Scalar Formula

```text
count = 0
peak = 0
sum_sq = 0
for sample in input:
  if is_finite(sample):
    count += 1
    peak = max(peak, abs(sample))
    sum_sq += sample * sample
return None if count == 0
```

RMS is computed by callers as:

```text
rms = sqrt(sum_sq / count)
```

## SIMD Pseudocode

```text
peak_v = 0
sum_v = 0
count = 0
for vector in chunks:
  mask = is_finite(vector)
  abs_v = abs(vector)
  peak_v = max(peak_v, select(mask, abs_v, 0))
  sum_v += widen_to_f64(select(mask, vector * vector, 0))
  count += count_true(mask)
reduce peak_v and sum_v
handle tail with scalar logic
return None if count == 0
```

## Numerical Contract

- Peak and count must match scalar exactly.
- Sum of squares may differ only by documented floating tolerance caused by
  reduction order.
- NaN and infinity are ignored for all three outputs.
- Empty or all-non-finite input returns `None`.
- Accumulator precision is `f64`.

## API Sketch

```rust
pub struct FinitePeakRms {
    pub finite_count: usize,
    pub peak: f32,
    pub sum_sq: f64,
}

pub fn finite_peak_rms_scalar(input: &[f32]) -> Option<FinitePeakRms>;
pub fn finite_peak_rms(input: &[f32]) -> Option<FinitePeakRms>;
```

## Tail And Fallback

- Tail lengths: cover vector width minus one, equal, and plus one.
- Forced scalar: available in tests.
- Forced SIMD: available where backend supports it.
- Unsupported target behavior: scalar fallback with no behavioral change.

## Tests

- exact scalar fixtures
- empty input
- all NaN/infinity
- one finite sample
- vector-width boundary lengths
- seeded random finite and mixed finite/non-finite data
- caller-level test through stats or normalize

## Benchmarks

- input sizes: 1K, 64K, and 10M samples
- benchmark command: use a fresh benchmark output directory
- baseline directory: never overwrite `target/benchmarks/sox_ng`
- pass/fail rule: correctness fails; performance is evidence until threshold is
  defined

## Done When

- Scalar reference is tested.
- SIMD matches scalar under forced backend tests.
- Tail handling is covered.
- At least one caller uses the backend-capable API.
