---
kind: oracle-notes
subject: "voice-activity"
status: planned
oracle_type: "modern-reference"
selected_reference: "WebRTC VAD plus Auralis edit-semantics tests"
rejected_references:
  - "SoX-ng VAD as default quality target"
owner: "effects/vad"
---

# Voice Activity Oracle

## One-Line Scope

This oracle covers frame-level speech activity detection and Auralis trim,
padding, and hangover semantics around detected speech regions.

## Why This Oracle

WebRTC VAD is a mature speech activity baseline with documented frame sizes,
sample-rate constraints, and aggressiveness levels. Auralis owns the editing
semantics around those decisions.

## Why Not The Rejected Reference

SoX-ng VAD uses a simple cepstral-power heuristic, can be fooled by non-speech
audio, and exposes awkward one-sided trim behavior. It should not define the
Auralis default.

## What The Oracle Proves

- audio property: speech-active regions are detected and edited according to
  Auralis mode parameters;
- parameter subset: frame length, aggressiveness, pre/post roll, min speech,
  min silence;
- input shape: labeled speech/silence fixtures;
- comparison signal: frame decisions and final trim boundaries;
- tolerance: frame-boundary tolerance, not sample equality to SoX-ng.

## What It Does Not Prove

It does not prove music/speech separation, every language/accent, SoX-ng parity,
or real-time streaming behavior.

## Fixtures

- generator or source: labeled speech/silence corpus plus deterministic
  synthetic boundaries;
- sample rate: WebRTC-supported rates;
- channels: mono and stereo fold-down;
- duration: short clips plus 10 min speech fixture;
- params: `aggressiveness=2`, `frame=20ms`, `pre_roll=100ms`,
  `post_roll=200ms`;
- expected artifact: frame labels and selected output region.

## Comparison Method

Compare WebRTC frame decisions where fixtures are generated from the reference,
then separately compare Auralis edit-region math against analytical expected
boundaries.

## Regeneration Rule

Regenerate when the WebRTC backend version, frame policy, or Auralis edit
semantics intentionally change.

## Done When

- VAD detection and edit semantics have separate tests.
- Unsupported frame and sample-rate combinations fail validation.
- SoX-ng is documented only as a legacy reference.
