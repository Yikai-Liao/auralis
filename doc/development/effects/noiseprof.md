---
kind: effect
effect: "noiseprof"
status: planned
owner: "crates/auralis-effects/src/noiseprof.rs"
graph_op: "noiseprof"
cli_example: "auralis render noise.wav -o profile.json --fx noiseprof"
family: "spectral-denoise"
oracle_policy: "modern-reference"
oracle: "doc/development/oracles/spectral-denoise.md"
---

# Noiseprof

## One-Line Description

`noiseprof` analyzes a noise-only audio region and writes a stationary spectral
noise profile for later `noisered` processing.

## Reference Target

- Primary reference: noisereduce-style stationary spectral-gating profile.
- Secondary reference: SoX-ng profile shape only if compatibility import/export
  is explicitly requested later.
- SoX-ng role: legacy profile-format reference, not the default algorithm
  target.
- Why this reference: stationary spectral gating maps directly to the
  `noiseprof -> noisered` workflow while allowing modern smoothing and
  threshold choices.
- Why not SoX-ng: SoX-ng is useful for legacy behavior but does not provide a
  strong modern denoise quality target.

## Parameters

| Param | Type | Default | Validation | Meaning |
| --- | --- | --- | --- | --- |
| `fft_size` | `usize` | `2048` | power of two, `>= 256` | STFT size |
| `hop` | `usize` | `fft_size / 4` | `> 0`, `<= fft_size` | STFT hop size |
| `window` | enum | `hann` | supported window | analysis window |
| `stat` | enum | `percentile` | `mean`, `median`, `percentile` | noise estimate statistic |
| `percentile` | `Percent` | `50%` | `0%..=100%` | percentile when `stat=percentile` |

## Signal Model

- Inputs: one noise-only audio port.
- Outputs: one profile artifact, not an audio stream.
- Changes length: not applicable.
- Changes sample rate: records input sample rate in the profile.
- Changes channels: records per-channel or shared-band statistics.
- Whole-buffer state: the profile is an aggregate over all analysis frames.

## Mathematics

For channel `c`, STFT frame `m`, and bin `k`:

```text
X_c[m,k] = STFT(x_c[n], fft_size, hop, window)
P_c[m,k] = abs(X_c[m,k])^2
noise_c[k] = statistic_m(P_c[m,k])
```

For percentile mode:

```text
noise_c[k] = percentile({ P_c[m,k] for all m }, percentile)
```

The profile stores `noise_c[k]`, optional spread estimates, sample rate,
FFT/hop/window settings, and channel policy.

## Pseudocode

```text
validate analysis params
for channel in channels:
  initialize per-bin accumulator
  for frame in stft_frames(channel):
    spectrum = fft(window * frame)
    for bin in bins:
      accumulator[bin].push(power(spectrum[bin]))
  profile[channel] = reduce_accumulators(accumulator, stat, percentile)
write profile with analysis metadata
```

## Implementation Notes

- Runtime type: `NoiseProfile`.
- Params type: `NoiseProfParams`.
- DSP helper: `stft-profile` should move to `auralis-dsp` if reused by
  `noisered`.
- Edge params: none.
- Reports: profile artifact and optional human summary.
- Profile serialization must be versioned.

## Performance Notes

- Use streaming accumulation inside the op implementation if it does not change
  graph-level whole-buffer semantics.
- Avoid storing all spectra when an online statistic is sufficient.
- Reuse FFT plans and scratch buffers.
- Benchmark mono and stereo noise clips from 10 s to 10 min.

## Validation And Diagnostics

- `fft_size` not a power of two.
- `hop == 0` or `hop > fft_size`.
- unsupported `window`.
- `percentile` outside `0%..=100%`.
- input has zero frames.

## Tests

- White-noise fixture produces finite non-negative bin estimates.
- Silence produces a zero or floor-limited profile according to policy.
- Profile metadata round-trips exactly.
- Invalid STFT params fail before execution.
- `noisered` rejects profiles with incompatible sample rate or FFT settings.

## Benchmarks

- Case: `noiseprof,fft_size=2048,hop=512,stat=percentile,percentile=50%`.
- Input shape: 48 kHz mono/stereo, 60 s stationary noise.
- Metric: profile generation time and peak memory.
- Baseline: first Rust implementation.
- Confirmation: rerun with `--skip-build` before claiming a win.

## Done When

- Profile format is versioned.
- STFT settings and statistics are documented.
- `noisered` compatibility checks are specified.
- Tests cover finite statistics, metadata, and invalid params.
