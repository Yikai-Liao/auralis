---
kind: oracle-notes
subject: "extreme-stretch"
status: planned
oracle_type: "modern-reference"
selected_reference: "PaulStretch / PaulXStretch"
rejected_references:
  - "normal stretch backend for extreme ambient factors"
owner: "future extreme-stretch effect"
---

# Extreme Stretch Oracle

## One-Line Scope

This oracle covers future ambient-scale stretching such as 8x, 20x, or larger
factors that should not be forced through the normal stretch backend.

## Why This Oracle

PaulStretch and PaulXStretch target extreme time stretching and spectral
smearing as a creative effect. That is a different product target from normal
musical tempo or pitch correction.

## Why Not The Rejected Reference

The normal stretch backend should not be required to handle extreme ambient
factors well. Keeping this as a separate oracle prevents `stretch` from gaining
unclear mode flags and incompatible quality expectations.

## What The Oracle Proves

- audio property: stable long-duration ambient smear without discontinuities;
- parameter subset: large factors such as 8x, 20x, and 50x;
- input shape: short tonal and noisy fixtures;
- comparison signal: duration, finite output, spectral smoothness summaries;
- tolerance: property-level and listening-oriented metrics.

## What It Does Not Prove

It does not prove normal musical stretch quality, tempo preservation, exact
sample equality, or future UI naming.

## Fixtures

- generator or source: deterministic tone, chord, and noise burst;
- sample rate: 48000;
- channels: stereo;
- duration: 5 seconds input;
- params: `factor=8`, `factor=20`;
- expected artifact: duration and spectral smoothness summaries.

## Comparison Method

Compare duration, finite output, and broad spectral distribution. Keep
listening checks as optional review notes until a measurable target is chosen.

## Regeneration Rule

Regenerate only when the future extreme-stretch effect is designed or the
selected reference version changes.

## Done When

- Extreme stretch is separate from standard `stretch`.
- Fixture factors and metrics are named before implementation begins.
