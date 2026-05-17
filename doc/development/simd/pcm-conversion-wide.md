---
kind: simd-kernel
kernel: "pcm-conversion-wide"
status: planned
priority: "A5"
owner: "crates/auralis-simd/src/convert_wide.rs"
scalar_reference: "pcm_conversion_scalar"
simd_backend: "rten-simd"
callers:
  - "auralis-wav"
  - "auralis-raw"
  - "format adapters"
---

# PCM Conversion Wide

## One-Line Description

Extend SIMD sample conversion beyond PCM16 to PCM8, PCM24, PCM32, and float32
validation/copy paths.

## SIMD Justification

Each sample conversion is independent after byte decoding and before output
packing. Validation masks can be accumulated per vector.

## Scalar Formula

```text
pcm8:  f32 = (u8 - 128) / 128
pcm24: f32 = sign_extend_24(bytes) / 8388608
pcm32: f32 = i32 / 2147483648
f32:   validate finite or preserve according to format policy
```

## SIMD Pseudocode

```text
for vector in chunks:
  raw = load_or_unpack(format_bytes)
  widened = convert_to_i32_or_f32(raw)
  scaled = widened * scale
  validation_mask |= invalid_mask(scaled)
  store(output, scaled)
handle tail with scalar conversion
```

## Numerical Contract

- Integer midpoint and full-scale mapping must match scalar exactly.
- PCM24 sign extension must match scalar for all byte orders.
- Float validation policy must match format docs.
- Saturation, rounding, and clipping rules for encode are explicit per format.

## API Sketch

```rust
pub fn pcm8_to_f32(input: &[u8], output: &mut [f32]);
pub fn pcm24_to_f32(input: &[u8], output: &mut [f32], endian: Endian);
pub fn pcm32_to_f32(input: &[u8], output: &mut [f32], endian: Endian);
pub fn validate_f32(input: &[f32]) -> ValidationSummary;
```

## Tail And Fallback

- PCM24 tails must account for 3-byte sample groups.
- Unsupported targets fall back to scalar conversion.
- Misaligned input is allowed if scalar accepts it.

## Tests

- min, max, zero, and midpoint fixtures for each format;
- endian fixtures for PCM24/PCM32;
- vector-width boundary sample counts;
- invalid float validation fixtures;
- decode/encode caller round trips.

## Benchmarks

- Input sizes: 1K, 64K, 10M samples.
- Cases: PCM8, PCM24, PCM32 decode and float32 validation.
- Baseline: existing scalar/PCM16 SIMD comparison.
- Confirmation: fresh output dir plus second `--skip-build` run.

## Done When

- Scalar conversion fixtures exist for each format.
- SIMD output matches scalar exactly where integer mapping is exact.
- Format adapters can opt into the backend-capable path.
