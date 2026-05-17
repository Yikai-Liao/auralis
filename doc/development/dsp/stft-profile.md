---
kind: dsp-primitive
primitive: "stft-profile"
status: planned
owner: "crates/auralis-dsp/src/stft_profile.rs"
extracted_from:
  - "noiseprof"
  - "noisered"
future_users:
  - "denoise experiments"
---

# STFT Profile

## One-Line Description

`stft_profile` provides reusable STFT windowing, spectral accumulation, and
profile metadata helpers for denoise effects.

## Why This Is A Primitive

`noiseprof` and `noisered` must agree on FFT size, hop, window, profile schema,
and bin layout. A shared primitive prevents incompatible effect-local STFT code.

## Mathematical Contract

```text
X_m[k] = FFT(window[n] * x[m*hop + n])
P_m[k] = abs(X_m[k])^2
profile[k] = statistic_m(P_m[k])
```

The statistic is selected by caller parameters.

## API Sketch

```rust
pub struct StftProfileConfig {
    pub fft_size: usize,
    pub hop: usize,
    pub window: WindowKind,
}

pub fn analyze_profile(input: &[f32], config: &StftProfileConfig) -> ProfileBins;
```

- allocation behavior: FFT scratch allocated once per analysis.
- panic/error behavior: validation rejects invalid FFT/hop/window.
- state object: FFT plan and scratch.
- backend selection: scalar/FFT-library first; SIMD not applicable first pass.

## Pseudocode

```text
validate config
for frame in overlapping_frames(input, hop, fft_size):
  windowed = frame * window
  spectrum = fft(windowed)
  accumulate power bins
reduce bins into profile statistic
```

## Callers

- `noiseprof`: build profile artifacts.
- `noisered`: validate and apply compatible profile settings.
- future denoise experiments.

## What Stays Outside

Noise-reduction mask policy, speech-denoise models, profile file I/O, graph
syntax, and CLI syntax.

## Numerical Notes

Window normalization, FFT scaling, power calculation, and profile statistic
must be documented so fixtures can regenerate deterministically.

## Performance Notes

Reuse FFT plans and scratch buffers. Avoid storing all spectra when an online
statistic is sufficient.

## Tests

Invalid config, silence, deterministic sine-bin fixture, profile metadata
round-trip, and caller-level `noiseprof/noisered` compatibility tests.

## Benchmarks

Benchmark 10 s, 60 s, and 10 min mono/stereo noise clips after the primitive is
shared.

## Done When

`noiseprof` and `noisered` share config and metadata validation, and tests
prove deterministic profile behavior.
