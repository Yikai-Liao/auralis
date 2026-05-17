---
kind: effect
effect: "noisered"
status: planned
owner: "crates/auralis-effects/src/noisered.rs"
graph_op: "noisered"
cli_example: "auralis render input.wav -o output.wav --fx noisered,profile=noise.profile,amount=0.7"
family: "spectral-denoise"
oracle_policy: "modern-reference"
oracle: "doc/development/oracles/spectral-denoise.md"
---

# Noisered

## One-Line Description

`noisered` reduces stationary background noise by applying a smoothed spectral
gate derived from a `noiseprof` profile.

## Reference Target

- Primary reference: noisereduce-style stationary spectral gating.
- Secondary reference: RNNoise/SpeexDSP only for a separate future
  speech-denoise effect.
- SoX-ng role: legacy behavior baseline only.
- Why this reference: stationary spectral gating preserves the existing
  profile-and-reduce workflow while giving a clearer modern algorithm target.
- Why not SoX-ng: SoX-ng `noisered` is a historical profile-based reducer and
  is only moderately effective for common noise cases.

## Parameters

| Param | Type | Default | Validation | Meaning |
| --- | --- | --- | --- | --- |
| `profile` | path/id | required | readable profile, compatible metadata | noise profile |
| `amount` | `f32` | `0.5` | `0..=1` | reduction strength |
| `floor` | `Decibels` | `-40dB` | `< 0dB` | maximum attenuation floor |
| `time_smoothing` | `Milliseconds` | `50ms` | `>= 0ms` | mask smoothing over time |
| `freq_smoothing` | `usize` | `2` | finite bin radius | mask smoothing over frequency |
| `lookahead` | `Milliseconds` | `0ms` | `>= 0ms` | optional future mask lookahead |

## Signal Model

- Inputs: one audio port and one noise profile artifact.
- Outputs: one audio port named `audio`.
- Changes length: no, except documented STFT edge padding policy.
- Changes sample rate: no.
- Changes channels: no.
- Whole-buffer state: overlapping STFT analysis/synthesis and smoothed mask
  state.

## Mathematics

For input STFT `X_c[m,k]` and profile power `N_c[k]`:

```text
P_c[m,k] = abs(X_c[m,k])^2
ratio_c[m,k] = max(P_c[m,k] - amount * N_c[k], 0) / max(P_c[m,k], epsilon)
raw_mask_c[m,k] = clamp(ratio_c[m,k], floor_gain, 1)
mask_c[m,k] = smooth_time_freq(raw_mask_c[m,k])
Y_c[m,k] = mask_c[m,k] * X_c[m,k]
y_c[n] = ISTFT(Y_c[m,k])
```

`floor_gain = db_to_linear(floor)`.

## Pseudocode

```text
load and validate profile metadata
prepare FFT, windows, smoothing state
for channel in channels:
  for frame in stft_frames(input[channel]):
    spectrum = fft(window * frame)
    for bin in bins:
      noise = profile[channel_or_shared][bin]
      raw_mask[bin] = compute_stationary_gate(spectrum[bin], noise, amount, floor)
    mask = smooth_mask(raw_mask, time_smoothing, freq_smoothing)
    reduced = mask * spectrum
    overlap_add(ifft(reduced), output[channel])
apply documented trim/pad policy to preserve input length
```

## Implementation Notes

- Runtime type: `NoiseRed`.
- Params type: `NoiseRedParams`.
- DSP helper: share STFT/window/profile code with `noiseprof`.
- Edge params: none.
- Reports: optional reduction summary only after core behavior is stable.
- Speech denoise is a separate effect and must not be hidden inside `noisered`.

## Performance Notes

- Reuse FFT plans and scratch buffers.
- Avoid per-frame heap allocation.
- Use planar per-channel processing to simplify smoothing state.
- SIMD is not first-pass priority; profile/mask algorithm correctness comes
  first.
- Benchmark stationary-noise music and speech fixtures.

## Validation And Diagnostics

- missing or unreadable `profile`.
- profile version unsupported.
- profile sample rate or FFT settings incompatible with input.
- `amount` outside `0..=1`.
- invalid attenuation `floor`.
- smoothing values that overflow frame or bin ranges.

## Tests

- Silence remains finite silence.
- `amount=0` returns input within overlap/add tolerance.
- Stationary noise fixture reduces measured noise-band energy.
- Profile metadata mismatch fails before execution.
- Reference fixtures compare denoise metrics against noisereduce-style outputs,
  not byte-for-byte SoX-ng output.

## Benchmarks

- Case: `noisered,amount=0.7,floor=-40dB`.
- Input shape: 48 kHz mono/stereo, 30 s and 300 s.
- Metric: render time, peak allocation, and noise-band reduction metric.
- Baseline: first Rust STFT implementation.
- Confirmation: rerun with `--skip-build` before claiming a win.

## Done When

- `noiseprof` and `noisered` share a versioned profile contract.
- STFT edge and length policy are deterministic.
- Tests cover validation, finite output, and reduction metrics.
- Speech denoise remains a separate documented future direction.
