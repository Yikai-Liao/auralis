---
kind: historical-roadmap
status: superseded
superseded_by:
  - effects.md
  - oracles.md
  - testing.md
---

# 6.x Effect Coverage Index

This file is the historical index for SoX-ng effect coverage. The detailed 6.x
milestone records are mechanically split under [`06-effects/`](06-effects/) so
each file stays reviewable. It is retained for implementation history and
coverage evidence, not as an authoritative planning surface.

Every effect feature must still follow the test contract in [`03-test-infrastructure.md`](03-test-infrastructure.md), the SIMD policy in [`04-simd.md`](04-simd.md), and the layered coverage gate from Feature 5.7.7.

## Current 6.x Execution Rules

- Implement or block exactly one leaf feature per gnhf iteration.
- Keep future effect planning in [`effects.md`](effects.md),
  [`effects/*.md`](effects/), [`oracles.md`](oracles.md), and
  [`oracles/*.md`](oracles/). Do not add new detailed plans here.
- README must not carry effect-by-effect status, exception notes, or compatibility matrices; put detailed status in this 6.x plan, golden manifests, or dedicated status docs.
- Treat each `06-effects/06-N-*.md` file as historical evidence for its
  milestone. Moving active effect plans now means updating the flat short
  effect and oracle documents, not this index.
- Before implementing any specialized, external-host, native-backed, or license-sensitive effect, run a feasibility/classification pass first and record one of: `implemented`, `partial`, `blocked`, or `not planned`.
- Specialized/native/license-sensitive effects must not enter normal implementation work until that classification says there is a MIT-compatible pure-Rust path with stable diagnostics and test coverage expectations.
- MIT-compatible pure Rust remains the default. GPL/native-wrapper paths must stay blocked or not planned unless the project policy changes.
- New sample-processing effects need a scalar reference, SIMD backend where the core loop is data-parallel, SoX-ng golden coverage where comparable, and explicit L0-L7 coverage metadata.

## Milestone Files

- [Milestone 6.1: complete existing SoX-ng semantics](06-effects/06-1-complete-existing-sox-ng-semantics.md)
- [Milestone 6.2: volume, level, and simple modulation effects](06-effects/06-2-volume-level-and-simple-modulation-effects.md)
- [Milestone 6.3: channel and mixing effects](06-effects/06-3-channel-and-mixing-effects.md)
- [Milestone 6.4: biquad and tone filters](06-effects/06-4-biquad-and-tone-filters.md)
- [Milestone 6.5: delay, echo, and modulation effects](06-effects/06-5-delay-echo-and-modulation-effects.md)
- [Milestone 6.6: sample-rate and time-domain effects](06-effects/06-6-sample-rate-and-time-domain-effects.md)
- [Milestone 6.7: dynamics, silence, and noise effects](06-effects/06-7-dynamics-silence-and-noise-effects.md)
- [Milestone 6.8: FIR, analysis, generation, and dither effects](06-effects/06-8-fir-analysis-generation-and-dither-effects.md)
- [Milestone 6.9: specialized and integration effects](06-effects/06-9-specialized-and-integration-effects.md)

## Current Blockers

- Feature 6.9.1 recorded `dolbyb` as blocked because SoX-ng routes it through GPLv2 `libdolbyb` C code and Auralis currently requires MIT-compatible pure Rust implementation paths. Feature 6.9.2 remains blocked by that decision.
- Feature 6.9.3 recorded `dop` as not planned for the effect registry because
  it is DSD-over-PCM transport packing, not PCM16 audio DSP. Reconsider it only
  in a future DSD/DoP format boundary.
- Feature 6.9.4 classified and implemented `earwax` as a pure-Rust,
  no-argument, stereo 44.1 kHz headphone-cue FIR with SoX-ng golden coverage and
  stable invalid-shape diagnostics.
- Feature 6.9.5 recorded `ladspa` as blocked because SoX-ng-compatible behavior
  requires an external native LADSPA plugin host, dynamic module loading through
  `LADSPA_PATH`, and plugin-dependent code, parameters, channel counts, latency,
  and licensing.
- Feature 6.9.6 recorded `sdm` as not planned for the effect registry because
  SoX-ng's SDM path emits 1-bit DSD-style output, depends on LGPL
  implementation details for filter tables and trellis behavior, and is not
  representable in the current PCM16 WAV golden harness. Feature 6.9.7 is not
  planned unless a future DSD/1-bit format boundary and clean-room
  MIT-compatible specification are added.
