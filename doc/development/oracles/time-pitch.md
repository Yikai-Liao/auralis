---
kind: oracle-notes
subject: "time-pitch"
status: planned
oracle_type: "modern-reference"
selected_reference: "Signalsmith Stretch; Rubber Band as quality reference"
rejected_references:
  - "SoX-ng stretch as default algorithm target"
  - "SoX-ng tempo/pitch as locked implementation target"
owner: "effects/stretch, effects/tempo, effects/pitch, effects/bend"
---

# Time And Pitch Oracle

## One-Line Scope

This oracle covers normal-range time stretching, tempo change, pitch shifting,
and bend effects.

## Why This Oracle

Signalsmith Stretch is the primary implementation reference because it is a
readable modern pitch/time design suitable for musical material. Rubber Band is
a quality reference for CI comparisons where it can remain an external tool
rather than a linked dependency.

## Why Not The Rejected Reference

SoX-ng `stretch` is a weak short-window cross-fade implementation. SoX-ng
`tempo` and `pitch` are useful baselines, but Auralis should not lock its
default quality to SoX-ng WSOLA behavior.

## What The Oracle Proves

- audio property: duration changes while pitch or tempo targets remain within
  tolerance;
- parameter subset: moderate factors such as 0.75x, 1.25x, and 2.0x;
- input shape: speech, monophonic tone, polyphonic music;
- comparison signal: duration, pitch estimate, transient metric, and decoded
  sample summaries;
- tolerance: metric-based quality tolerance, not sample equality.

## What It Does Not Prove

It does not prove extreme ambient stretch, formant preservation, exact Rubber
Band output, every sample rate, or performance.

## Fixtures

- generator or source: deterministic sine, click train, speech, and music
  fixtures;
- sample rate: 48000;
- channels: mono and stereo;
- duration: 10 to 30 seconds;
- params: `factor=0.75`, `factor=1.25`, `factor=2.0`;
- expected artifact: duration, pitch, transient, and finite-output summaries.

## Comparison Method

Use metric comparison for duration, pitch stability, transient preservation,
and finite output. Do not require exact sample equality against Signalsmith or
Rubber Band unless a direct port intentionally adopts their numeric details.

## Regeneration Rule

Regenerate fixtures only when the time/pitch target semantics, reference
version, or metric tolerances intentionally change.

## Done When

- Stretch, tempo, pitch, and bend name their specific use of this oracle.
- Rubber Band remains an external quality reference, not a linked default
  dependency.
- Extreme stretch is covered by a separate oracle note.
