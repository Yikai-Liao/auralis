---
kind: oracle-notes
subject: "modulation-delay"
status: planned
oracle_type: "modern-reference"
selected_reference: "Faust phaflangers.lib plus DaisySP modulation effects"
rejected_references:
  - "SoX-ng phaser as true phaser oracle"
owner: "effects/phaser, effects/chorus, effects/flanger"
---

# Modulation Delay Oracle

## One-Line Scope

This oracle covers phaser, chorus, flanger, and future delay-modulation effects
where SoX-ng parameter compatibility is useful but not a sufficient quality
target.

## Why This Oracle

Faust `phaflangers.lib` gives documented phaser and flanger structures, while
DaisySP provides compact chorus/flanger/phaser implementations useful for API,
stability, and listening sanity checks. Together they cover the intended modern
delay/modulation family better than a single SoX-ng baseline.

## Why Not The Rejected Reference

SoX-ng `phaser` is not selected because its behavior is flanger-like rather
than a true all-pass phaser. SoX-ng chorus/flanger may still inform parameter
compatibility, but its default interpolation quality should not define Auralis'
default sound.

## What The Oracle Proves

- audio property: moving notches or delay-modulation coloration are in the
  expected family;
- parameter subset: rate, depth, feedback, delay/sweep width, and wet/dry mix;
- input shape: deterministic impulse, sine sweep, and stereo music snippets;
- comparison signal: decoded samples plus spectral summaries;
- tolerance: property and metric tolerance, not byte-for-byte equality.

## What It Does Not Prove

It does not prove exact sample equality, every legacy SoX parameter, graph
validation, CLI syntax, or performance.

## Fixtures

- generator or source: deterministic impulse, sine sweep, and short music clip;
- sample rate: 48000;
- channels: mono and stereo;
- duration: 10 seconds for reference fixtures;
- params: one canonical fixture per effect;
- expected artifact: decoded samples plus spectral notch or comb summary.

## Comparison Method

Compare finite output, broad spectral movement, and effect-family metrics. Do
not require exact equality unless Auralis intentionally ports a specific
reference implementation.

## Regeneration Rule

Regenerate fixtures only when the Auralis target semantics change or the
selected reference version is intentionally updated. Record reference version,
fixture command, and metric diff summary.

## Done When

- Each modulation effect names whether it uses Faust, DaisySP, or SoX-ng compat.
- Fixture params are reproducible.
- Tolerance reflects property-level comparison.
