# 6.x Effect Coverage Index

This file is the lightweight index for SoX-ng effect coverage. The detailed 6.x milestone plans are mechanically split under [`06-effects/`](06-effects/) so each file stays reviewable.

Every effect feature must still follow the test contract in [`03-test-infrastructure.md`](03-test-infrastructure.md), the SIMD policy in [`04-simd.md`](04-simd.md), and the layered coverage gate from Feature 5.7.7.

## Current 6.x Execution Rules

- Implement or block exactly one leaf feature per gnhf iteration.
- Keep this file as the index and cross-milestone policy surface. Effect-by-effect details belong in the matching `06-effects/06-N-*.md` file, not in README.
- README must not carry effect-by-effect status, exception notes, or compatibility matrices; put detailed status in this 6.x plan, golden manifests, or dedicated status docs.
- Treat each `06-effects/06-N-*.md` file as the authoritative plan for its milestone. Moving an effect between files must update this index and preserve one leaf-feature-per-gnhf iteration.
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
- The next plan adjustment should continue in Milestone 6.9 by classifying
  `ladspa` and `sdm` before implementation work resumes.
