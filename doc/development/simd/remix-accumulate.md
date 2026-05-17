---
kind: simd-kernel
kernel: "remix-accumulate"
status: planned
priority: "A4"
owner: "crates/auralis-simd/src/remix.rs"
scalar_reference: "remix_accumulate_scalar"
simd_backend: "rten-simd"
callers:
  - "remix"
  - "channels"
  - "mix"
---

# Remix Accumulate

## One-Line Description

Accelerate channel remix accumulation: `output_channel += input_channel * gain`.

## SIMD Justification

For a fixed input/output channel pair, each frame accumulation is independent
except for same-index output read/modify/write.

## Scalar Formula

```text
for n in frames:
  out[n] += input[n] * gain
if clamp:
  out[n] = clamp(out[n], -1, 1)
```

## SIMD Pseudocode

```text
for mapping in channel_mappings:
  gain_v = splat(mapping.gain)
  for vector in chunks:
    out_v = load(output_channel)
    in_v = load(input_channel)
    out_v = out_v + in_v * gain_v
    store(output_channel, out_v)
if clamp:
  run clamp kernel over output channels
handle tails with scalar loop
```

## Numerical Contract

- Mapping order must match scalar exactly.
- FMA use must be reflected in tolerance or disabled for exact parity.
- Clamp policy must match scalar.
- NaN and infinity propagate according to scalar arithmetic until validation or
  output guards handle them.

## API Sketch

```rust
pub fn remix_accumulate(
    output: &mut [f32],
    input: &[f32],
    gain: f32,
    backend: BackendSelection,
);
```

## Tail And Fallback

- Tail uses scalar accumulation.
- Unsupported targets fall back to scalar.
- Aliasing between input and output must be forbidden or explicitly handled.

## Tests

- mono-to-stereo and stereo-to-mono mappings;
- zero gain and negative gain;
- multiple mappings into one output channel;
- vector-width boundary lengths;
- seeded random channel matrices;
- caller-level remix/channel conversion tests.

## Benchmarks

- Cases: mono-to-stereo, stereo-to-mono, 6-to-2 channel remix.
- Input sizes: 30 s and 300 s.
- Baseline: scalar remix loop.
- Confirmation: fresh output dir plus second `--skip-build` run.

## Done When

- Mapping order parity is tested.
- SIMD kernel handles tail and accumulation safely.
- Remix callers can select scalar or SIMD for differential tests.
