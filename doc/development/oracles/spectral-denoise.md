---
kind: oracle-notes
subject: "spectral-denoise"
status: planned
oracle_type: "modern-reference"
selected_reference: "noisereduce-style stationary spectral gating"
rejected_references:
  - "SoX-ng noisered as default quality target"
  - "RNNoise hidden inside noisered"
owner: "effects/noiseprof, effects/noisered"
---

# Spectral Denoise Oracle

## One-Line Scope

This oracle covers stationary `noiseprof -> noisered` spectral-gating behavior.

## Why This Oracle

Stationary spectral gating preserves the useful profile-and-reduce workflow
while giving Auralis clear STFT, threshold, smoothing, and mask semantics.

## Why Not The Rejected Reference

SoX-ng `noisered` is a legacy profile-based reducer and should not be the
default quality target. RNNoise is a speech-denoise model and should be a
separate effect, not a hidden implementation mode of `noisered`.

## What The Oracle Proves

- audio property: stationary noise-band energy is reduced while output remains
  finite;
- parameter subset: FFT size, hop, amount, floor, time and frequency smoothing;
- input shape: noise-only profile clip plus noisy speech/music clip;
- comparison signal: noise-band reduction, speech/music preservation summary,
  decoded sample bounds;
- tolerance: metric tolerance, not exact sample equality.

## What It Does Not Prove

It does not prove non-stationary speech denoise, RNNoise behavior, every noise
type, subjective quality, or exact SoX-ng compatibility.

## Fixtures

- generator or source: deterministic stationary hiss/hum plus speech/music;
- sample rate: 48000;
- channels: mono and stereo;
- duration: 10 s noise profile, 30 s noisy signal;
- params: `fft_size=2048`, `hop=512`, `amount=0.7`, `floor=-40dB`;
- expected artifact: profile metadata, decoded output, and spectral metrics.

## Comparison Method

Compare profile metadata, finite output, and spectral reduction metrics. Use
sample-level comparison only for deterministic internal round trips such as
profile serialization.

## Regeneration Rule

Regenerate fixtures when STFT settings, profile schema, reference version, or
metric tolerances intentionally change.

## Done When

- `noiseprof` and `noisered` share a versioned profile contract.
- Stationary denoise and speech denoise remain separate plans.
- Metrics reflect what spectral gating can actually prove.
