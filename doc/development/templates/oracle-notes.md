---
kind: oracle-notes
subject: "<effect-or-primitive>"
status: planned
oracle_type: "<sox-ng|modern-reference|analytical|none>"
selected_reference: "<reference>"
rejected_references:
  - "<reference>"
owner: "<area>"
---

# Oracle Notes Template

Use this to document why a reference is valid for one effect, primitive, or
format boundary. Put document metadata in the YAML front matter above. Keep the
filled file under 250 lines.

## One-Line Scope

Example:

The phaser oracle checks that Auralis targets true all-pass phasing rather than
SoX-ng's flanger-like phaser behavior.

## Why This Oracle

Explain what makes the selected reference useful.

Example:

Faust exposes a documented true-phaser structure with all-pass notch placement,
notch count, depth, feedback, and stereo variants. Those controls map to the
Auralis target semantics.

## Why Not The Rejected Reference

Required when the oracle is not SoX-ng.

Example:

SoX-ng is not selected because its `phaser` behavior is flanger-like and does
not define the true phaser algorithm Auralis wants to implement.

## What The Oracle Proves

- audio property:
- parameter subset:
- input shape:
- comparison signal:
- tolerance:

Example:

The oracle proves that notch movement and spectral coloration are in the
expected family for a fixed-rate stereo sine sweep and an impulse fixture.

## What It Does Not Prove

Example:

It does not prove byte-for-byte equality, CLI syntax, graph validation, every
sample rate, or performance.

## Fixtures

- generator or source:
- sample rate:
- channels:
- duration:
- params:
- expected artifact:

Example:

- generator: deterministic sine sweep plus impulse
- sample rate: 48000
- channels: 2
- duration: 10 seconds
- params: `phaser,notches=6,depth=80%,rate=0.5Hz,feedback=0.4`
- expected artifact: decoded samples and spectral summary

## Comparison Method

Example:

Compare spectral notch locations and finite output bounds. Do not require exact
sample equality against Faust unless the implementation is intentionally
ported with identical numeric details.

## Regeneration Rule

Example:

Regenerate fixtures only when the Auralis phaser target semantics change or the
reference implementation version is intentionally updated. Record the command,
reference version, and sample diff summary.

## Done When

- Selected reference is named.
- Rejected references have reasons.
- Fixture shape and params are reproducible.
- Tolerance matches what the oracle can actually prove.
