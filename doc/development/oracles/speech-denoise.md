---
kind: oracle-notes
subject: "speech-denoise"
status: planned
oracle_type: "modern-reference"
selected_reference: "RNNoise and SpeexDSP"
rejected_references:
  - "mixing speech denoise into noisered"
owner: "future speech-denoise effect"
---

# Speech Denoise Oracle

## One-Line Scope

This oracle is reserved for a future speech-denoise effect and is not part of
`noisered`.

## Why This Oracle

RNNoise and SpeexDSP are speech-focused references. They fit voice cleanup
better than stationary profile-based spectral gating.

## Why Not The Rejected Reference

`noisered` should keep the profile-and-reduce stationary-noise contract. Hiding
speech denoise inside it would make validation, parameters, and quality tests
ambiguous.

## What The Oracle Proves

- audio property: speech noise suppression on labeled voice fixtures;
- parameter subset: aggressiveness or model choice, if exposed;
- input shape: speech with controlled background noise;
- comparison signal: speech/noise metrics and finite output;
- tolerance: metric and listening-review tolerance.

## What It Does Not Prove

It does not prove stationary `noisered` behavior, music denoise, exact model
parity, or public API design.

## Fixtures

- generator or source: future labeled speech/noise corpus;
- sample rate: backend-supported speech rates;
- channels: mono first;
- duration: short and long speech clips;
- params: to be selected when the effect is planned;
- expected artifact: denoised output and speech/noise metrics.

## Comparison Method

Use speech-focused objective metrics and curated listening checks. Do not
compare against SoX-ng `noisered`.

## Regeneration Rule

Regenerate only when a speech-denoise effect enters active planning.

## Done When

- Speech denoise has its own effect name and parameter contract.
- The oracle is not referenced as default `noisered` behavior.
