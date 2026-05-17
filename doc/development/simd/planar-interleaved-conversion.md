---
kind: simd-kernel
kernel: "planar-interleaved-conversion"
status: planned
priority: "A6"
owner: "crates/auralis-simd/src/interleave.rs"
scalar_reference: "interleave_scalar"
simd_backend: "rten-simd"
callers:
  - "wav writer"
  - "format adapters"
  - "mono/stereo fast paths"
---

# Planar Interleaved Conversion

## One-Line Description

Accelerate mono/stereo planar-to-interleaved and interleaved-to-planar sample
layout conversion.

## SIMD Justification

Layout conversion is a deterministic shuffle. Mono copy and stereo deinterleave
or interleave can use vector loads, stores, and lane shuffle patterns.

## Scalar Formula

```text
interleave stereo:
  out[2*n] = left[n]
  out[2*n + 1] = right[n]

deinterleave stereo:
  left[n] = input[2*n]
  right[n] = input[2*n + 1]
```

## SIMD Pseudocode

```text
for vector frames:
  left_v = load(left)
  right_v = load(right)
  lo, hi = interleave_lanes(left_v, right_v)
  store(out, lo)
  store(out + vector_width, hi)
handle tail frames with scalar loop
```

## Numerical Contract

- Values are copied exactly.
- NaN payload preservation follows backend load/store behavior; no arithmetic
  is performed.
- Channel order must match scalar exactly.
- Empty buffers are no-ops.

## API Sketch

```rust
pub fn interleave_stereo(left: &[f32], right: &[f32], out: &mut [f32]);
pub fn deinterleave_stereo(input: &[f32], left: &mut [f32], right: &mut [f32]);
pub fn copy_mono(input: &[f32], out: &mut [f32]);
```

## Tail And Fallback

- Tail is measured in frames, not samples.
- Unsupported channel counts use scalar generic layout conversion.
- Unsupported targets fall back to scalar.

## Tests

- mono, stereo, empty, one-frame, and odd-frame fixtures;
- vector-width boundary frame counts;
- NaN payload smoke fixture where feasible;
- caller-level WAV writer and decoder layout tests.

## Benchmarks

- Input sizes: 1K, 64K, 10M frames.
- Cases: stereo interleave, stereo deinterleave, mono copy.
- Baseline: scalar layout conversion.
- Confirmation: fresh output dir plus second `--skip-build` run.

## Done When

- Fast paths are explicit for mono and stereo.
- Generic channel conversion remains correct for other layouts.
- WAV writer or format adapters use the backend-capable path.
