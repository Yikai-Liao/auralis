---
kind: dsp-primitive
primitive: "fractional-delay"
status: planned
owner: "crates/auralis-dsp/src/fractional_delay.rs"
extracted_from:
  - "chorus"
  - "flanger"
future_users:
  - "phaser"
  - "vibrato"
---

# Fractional Delay

## One-Line Description

`fractional_delay` reads delayed samples at fractional positions using a
documented interpolation kernel and per-render delay state.

## Why This Is A Primitive

Chorus, flanger, and future vibrato need delay-line interpolation. Keeping the
interpolation contract shared prevents each effect from inventing different
edge and stability behavior.

## Mathematical Contract

For delay `d = i + frac`:

```text
y[n] = interpolate(x[n - i], x[n - i - 1], frac)
```

The first version should choose linear or cubic interpolation explicitly and
document boundary samples.

## API Sketch

```rust
pub struct FractionalDelayLine { /* private */ }

impl FractionalDelayLine {
    pub fn new(max_delay_frames: usize) -> Self;
    pub fn push_read(&mut self, input: f32, delay_frames: f32) -> f32;
}
```

- allocation behavior: allocate state once per render/channel.
- panic/error behavior: validation rejects delay outside max range.
- state object: delay line.
- backend selection: scalar first; SIMD not applicable first pass.

## Pseudocode

```text
validate delay <= max_delay
write input into ring buffer
read_pos = write_pos - delay
base = floor(read_pos)
frac = read_pos - base
return interpolate(buffer[base], buffer[base - 1], frac)
```

## Callers

- chorus and flanger variable-delay reads;
- future vibrato;
- phaser only if a compat flanger-like mode needs it.

## What Stays Outside

LFO generation, feedback policy, wet/dry mix, graph execution, and CLI syntax.

## Numerical Notes

Interpolation formula, boundary handling, and denormal policy must be stable
and testable.

## Performance Notes

Use contiguous ring-buffer storage and avoid allocation inside sample loops.
Optimize algorithm/layout before considering SIMD.

## Tests

Integer delay, half-sample delay, boundary delay, max-delay validation, impulse
response, and caller-level chorus/flanger fixtures.

## Benchmarks

Benchmark long stereo chorus/flanger-style reads after the scalar primitive is
used by at least one caller.

## Done When

Interpolation and boundary contracts are explicit, scalar tests pass, and
effect callers stop owning duplicate delay-line code.
