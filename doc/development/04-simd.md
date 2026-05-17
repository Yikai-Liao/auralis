---
kind: historical-roadmap
status: superseded
superseded_by:
  - simd.md
  - dsp.md
---

# 4.x SIMD Foundation Milestone

SIMD is part of the normal implementation path after Feature 3.3, not a late
optimization phase. Every new sample-processing feature must ship with a scalar
reference path and a SIMD backend when the core loop is data-parallel. If SIMD
is not applicable, the feature must document why.
This file is retained for policy history and acceptance traceability; new SIMD
kernel planning belongs in `simd.md` and `simd/*.md`.

## Milestone 4.1: backend contract and dispatch

### Feature 4.1.1: backend trait skeleton

Status: implemented.

Acceptance tests:

- scalar backend implements the backend trait;
- public library APIs do not expose backend crate types;
- placeholder SIMD backend compiles where the selected backend crate is
  available.

### Feature 4.1.2: deterministic backend selection

Status: implemented.

Implement named backends:

- `scalar`
- `simd`

Acceptance tests:

- tests can force scalar;
- tests can force SIMD when available;
- unsupported SIMD platforms fall back to scalar with a documented reason;
- backend choice does not change public APIs.

### Feature 4.1.3: scalar-vs-SIMD conformance helpers

Status: implemented.

Acceptance tests:

- helpers compare exact output and tolerance-based output;
- helpers report backend name, case ID, first failing index, and error metrics;
- helpers can run the same case under forced scalar and forced SIMD.

## Milestone 4.2: SIMD sample conversion

### Feature 4.2.1: SIMD `i16_to_f32`

Status: implemented.

Acceptance tests:

- exact known conversions;
- empty, one-sample, odd-length, and vector-tail lengths;
- seeded random buffers across the full PCM16 range;
- scalar-vs-SIMD output equality;
- WAV decode tests pass under forced scalar and forced SIMD.

### Feature 4.2.2: SIMD `f32_to_i16`

Status: implemented.

Acceptance tests:

- exact known conversions;
- clipping and rounding documented;
- NaN and infinity behavior documented;
- empty, one-sample, odd-length, and vector-tail lengths;
- seeded random buffers including near-clipping values;
- scalar-vs-SIMD output equality or documented one-LSB tolerance;
- WAV encode tests pass under forced scalar and forced SIMD.

## Milestone 4.3: SIMD retrofit for implemented basic effects

### Feature 4.3.1: SIMD `gain` retrofit

Status: implemented.

Acceptance tests:

- existing analytical and SoX-ng golden tests pass unchanged;
- scalar and SIMD outputs match for deterministic fixtures;
- random buffers cover silence, near-clipping values, NaN, and infinities where
  public behavior is defined;
- CLI output is identical or within documented tolerance under forced scalar and
  forced SIMD.

### Feature 4.3.2: SIMD `dcshift` retrofit

Status: implemented.

Acceptance tests:

- existing analytical and SoX-ng golden tests pass unchanged;
- scalar and SIMD outputs match for deterministic fixtures;
- random buffers cover silence, denormals, near-clipping values, NaN, and
  infinities where public behavior is defined;
- CLI output is identical or within documented tolerance under forced scalar and
  forced SIMD.

### Feature 4.3.3: SIMD linear `fade` retrofit

Status: implemented.

Acceptance tests:

- existing analytical and SoX-ng golden tests pass unchanged;
- scalar and SIMD outputs match for deterministic fade-in, fade-out, and
  combined fade fixtures;
- CLI output is identical or within documented tolerance under forced scalar and
  forced SIMD.

## When to add SIMD

For every new sample-processing feature:

1. define the scalar reference behavior first;
2. add or extend the backend trait for the effect's core kernel;
3. implement the SIMD backend in the same feature when the kernel is
   data-parallel;
4. add forced scalar and forced SIMD tests;
5. add scalar-vs-SIMD differential tests;
6. document a SIMD N/A reason only when the algorithm has no useful vectorizable
   kernel.

Benchmarks are required for performance-sensitive kernels, but lack of a
benchmark is not a reason to skip the SIMD backend for an otherwise vectorizable
effect.
