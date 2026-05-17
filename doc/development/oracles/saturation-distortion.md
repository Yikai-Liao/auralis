---
kind: oracle-notes
subject: "saturation-distortion"
status: planned
oracle_type: "modern-reference"
selected_reference: "Auralis curves, DaisySP, MusicDSP, Airwindows listening references"
rejected_references:
  - "SoX-ng as default creative distortion target"
owner: "effects/overdrive, effects/saturation, effects/contrast"
---

# Saturation And Distortion Oracle

## One-Line Scope

This oracle covers creative nonlinear effects where Auralis owns the transfer
curve semantics.

## Why This Oracle

DaisySP and MusicDSP provide small, inspectable distortion structures and
formula references. Airwindows is useful as a creative listening reference, but
Auralis curves must be explicitly documented.

## Why Not The Rejected Reference

SoX-ng can be a baseline, but creative nonlinear effects should not be defined
by one legacy waveshaper.

## What The Oracle Proves

Curve shape, finite bounded output where promised, harmonic-growth summaries,
and parameter monotonicity.

## What It Does Not Prove

Subjective preference, exact Airwindows parity, every possible curve variant,
or SIMD approximation correctness.

## Fixtures

Use deterministic sine, swept sine, impulse, and short music fixtures with
default and high-drive settings.

## Comparison Method

Compare transfer-curve samples, output bounds, harmonic summaries, and finite
output. Listening review is optional supporting evidence.

## Regeneration Rule

Regenerate when a curve formula, approximation, or selected reference version
changes.

## Done When

Each nonlinear effect names its curve and the reference role.
