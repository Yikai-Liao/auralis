---
kind: simd-kernel
kernel: "<kernel-name>"
status: planned
priority: "<A1|A2|A3|A4|A5|A6|A7>"
owner: "crates/auralis-simd/src/<kernel>.rs"
scalar_reference: "<scalar-function>"
simd_backend: "rten-simd"
callers:
  - "<caller>"
---

# SIMD Kernel Template

Use this for one vectorizable kernel. Put document metadata in the YAML front
matter above. Keep the filled file under 250 lines.

## One-Line Description

Example:

Compute finite sample count, absolute peak, and RMS sum over an `f32` slice.

## SIMD Justification

Explain why lanes are independent.

Example:

Each sample contributes independently to finite count, peak, and sum of squares.
Partial lane accumulators can be reduced after the vector loop.

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
rms = sqrt(sum_sq / count)
```

## SIMD Pseudocode

```text
for vector in chunks:
  mask = is_finite(vector)
  abs_v = abs(vector)
  peak_v = max(peak_v, select(mask, abs_v, 0))
  sum_v += select(mask, vector * vector, 0)
  count_v += mask_to_count(mask)
reduce peak_v, sum_v, count_v
handle tail with scalar reference logic
```

## Numerical Contract

- peak exactness:
- RMS tolerance:
- NaN/infinity policy:
- empty/all-non-finite behavior:
- accumulator precision:

Example:

Peak and count must match scalar exactly. RMS may use documented floating
tolerance if vector reduction order differs.

## API Sketch

```rust
pub struct FinitePeakRms {
    pub finite_count: usize,
    pub peak: f32,
    pub sum_sq: f64,
}

pub fn finite_peak_rms_scalar(input: &[f32]) -> Option<FinitePeakRms>;
pub fn finite_peak_rms_with_backend(input: &[f32], backend: BackendSelection)
    -> Option<FinitePeakRms>;
```

## Tail And Fallback

- tail lengths:
- forced scalar:
- forced SIMD:
- unsupported target behavior:

Example:

Unsupported SIMD falls back to scalar and records backend fallback reason. Tail
handling uses the scalar loop over the remaining samples.

## Tests

- exact scalar fixtures
- empty and all-non-finite
- one sample
- vector width minus one, equal, plus one
- seeded random data
- forced scalar vs requested SIMD
- caller-level test through stats/normalize

## Benchmarks

- input sizes:
- benchmark command:
- baseline directory:
- pass/fail or report-only rule:

Example:

Benchmark 1K, 64K, and 10M sample buffers. Write reports to a fresh directory
and confirm speedups with a second skip-build run.

## Done When

- Scalar reference is tested.
- SIMD matches scalar under forced backend tests.
- Tail handling is covered.
- Callers use the backend-capable API.
- Benchmark evidence exists or the kernel is marked correctness-only.
