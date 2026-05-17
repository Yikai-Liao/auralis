---
kind: dsp-primitive
primitive: "finite-peak-rms"
status: planned
owner: "crates/auralis-dsp/src/stats.rs"
extracted_from:
  - "stats"
  - "normalize"
future_users:
  - "write_validation"
---

# Finite Peak RMS

## One-Line Description

`finite_peak_rms` scans samples and returns finite count, absolute peak, and
sum of squares.

## Why This Is A Primitive

Stats, normalization, safety guards, and writer validation repeatedly need the
same scan. A shared primitive keeps NaN/Infinity policy consistent.

## Mathematical Contract

```text
count = number of finite x_i
peak = max(abs(x_i)) over finite x_i
sum_sq = sum(x_i * x_i) over finite x_i
```

Return `None` when `count == 0`.

## API Sketch

```rust
pub struct FinitePeakRms {
    pub finite_count: usize,
    pub peak: f32,
    pub sum_sq: f64,
}

pub fn finite_peak_rms(input: &[f32]) -> Option<FinitePeakRms>;
```

- allocation behavior: no allocation.
- panic/error behavior: none.
- state object: none.
- backend selection: scalar first, SIMD through `finite-peak-rms`.

## Pseudocode

```text
for sample in input:
  if is_finite(sample):
    count += 1
    peak = max(peak, abs(sample))
    sum_sq += sample * sample
return None if count == 0
```

## Callers

- `stats`: report finite sample properties;
- normalize: peak/RMS analysis;
- writer validation: reject or report non-finite output.

## What Stays Outside

Normalization gain choice, output repair, report formatting, and CLI syntax.

## Numerical Notes

Peak and count are exact relative to scalar. `sum_sq` uses `f64`; SIMD may need
a documented tolerance for reduction order.

## Performance Notes

This is a high-fanout scan and should avoid multiple passes when callers need
both peak and RMS.

## Tests

Empty, all non-finite, single finite sample, mixed finite/non-finite, seeded
random data, and caller-level stats/normalize validation.

## Benchmarks

Run 1K, 64K, and 10M sample buffers. Confirm any SIMD win with a second
`--skip-build` run.

## Done When

The primitive is the shared source for peak/RMS scans and has scalar plus SIMD
differential coverage.
