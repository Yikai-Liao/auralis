---
kind: oracle-notes
subject: "reverb"
status: planned
oracle_type: "modern-reference"
selected_reference: "Faust reverbs with Freeverb as compatibility baseline"
rejected_references:
  - "SoX-ng Freeverb as sole default target"
owner: "effects/reverb"
---

# Reverb Oracle

## One-Line Scope

This oracle covers default Auralis reverb quality and a separate compatibility
baseline for Freeverb-style behavior.

## Why This Oracle

Faust reverbs document Schroeder, Moorer, Freeverb, and FDN-style families, so
they give Auralis room to choose a modern default while keeping Freeverb as a
small baseline.

## Why Not The Rejected Reference

SoX-ng reverb is useful but Freeverb-style behavior should not be the only
Auralis target.

## What The Oracle Proves

Impulse decay, finite output, stereo decorrelation, broad spectral decay, and
tail policy for selected parameter fixtures.

## What It Does Not Prove

Exact sample equality, all room models, all sample rates, or subjective quality
without listening review.

## Fixtures

Use deterministic impulse, burst, and short music fixtures at 48 kHz stereo
with default and long-decay settings.

## Comparison Method

Compare decay envelope, finite bounds, stereo width metrics, and spectral
summaries. Use exact comparison only for Auralis-owned deterministic fixtures.

## Regeneration Rule

Regenerate when the selected default reverb family, parameters, or reference
version changes.

## Done When

Default and compatibility reverb targets are named separately.
