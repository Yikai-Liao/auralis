---
kind: simd-kernel
kernel: "finite-peak-rms"
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

# Finite Peak RMS

## One-Line Description

Compute finite sample count, absolute peak, and sum of squares over an `f32`
slice.

## SIMD Justification

Each sample contributes independently to finite count, peak, and sum of
squares. Lane accumulators reduce after the vector loop.

## Scalar Formula

```text
count = 0
peak = 0
sum_sq = 0
for x in input:
  if is_finite(x):
    count += 1
    peak = max(peak, abs(x))
    sum_sq += x * x
return None if count == 0
```

## SIMD Pseudocode

```text
for vector in chunks:
  mask = is_finite(vector)
  peak_v = max(peak_v, select(mask, abs(vector), 0))
  sum_v += widen_to_f64(select(mask, vector * vector, 0))
  count += count_true(mask)
reduce peak_v and sum_v
handle tail with scalar loop
```

## Numerical Contract

- Peak and count match scalar exactly.
- `sum_sq` may differ only by documented floating reduction tolerance.
- NaN and infinity are ignored.
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

- Tail lengths: vector width minus one, equal, plus one.
- Forced scalar and requested SIMD must be testable.
- Unsupported targets fall back to scalar with identical behavior.

## Tests

- empty and all-non-finite input;
- one finite sample;
- vector-width boundary lengths;
- seeded random finite and mixed finite/non-finite data;
- caller-level tests through stats, normalize, or write validation.

## Benchmarks

- Input sizes: 1K, 64K, 10M samples.
- Baseline: current scalar scan.
- Output directory: fresh benchmark directory, never `target/benchmarks/sox_ng`.
- Confirmation: rerun with `--skip-build` before claiming a win.

## Done When

- Scalar reference is tested.
- SIMD matches scalar under forced backend tests.
- Tail handling is covered.
- At least one caller uses the backend-capable API.
