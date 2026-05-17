---
kind: index
status: active
---

# SIMD Kernel Index

This index tracks SIMD kernel documents. It is a navigation and priority file
only. Put scalar formulas, vector pseudocode, numerical contracts, tests, and
benchmark details in `doc/development/simd/<kernel>.md`.

## Columns

| Column | Meaning |
| --- | --- |
| Kernel | SIMD target name |
| Priority | A1, A2, A3, or later priority |
| Status | shared status value |
| Callers | effects, DSP primitives, codecs, or validation paths |
| Backend | selected SIMD backend |
| Detail | link to the kernel document |

## Migration Table

| Kernel | Priority | Status | Callers | Backend | Detail |
| --- | --- | --- | --- | --- | --- |
| `finite-peak-rms` | A1 | planned | stats, normalize, write validation | `rten-simd` | `simd/finite-peak-rms.md` |
| `axpy-scale-mul-clamp` | A2 | planned | DSP primitives, mix, remix, wet/dry paths | `rten-simd` | `simd/axpy-scale-mul-clamp.md` |
| `tremolo-modulation` | A3 | planned | tremolo channel modulation multiply | `rten-simd` | `simd/tremolo-modulation.md` |
| `remix-accumulate` | A4 | planned | remix scale, accumulate, clamp | `rten-simd` | `simd/remix-accumulate.md` |
| `pcm-conversion-wide` | A5 | planned | PCM8, PCM24, PCM32, float32 validation/copy | `rten-simd` | `simd/pcm-conversion-wide.md` |
| `planar-interleaved-conversion` | A6 | planned | WAV writer, format adapters, mono/stereo fast paths | `rten-simd` | `simd/planar-interleaved-conversion.md` |
| `fixed-direct-fir` | A7 | planned | fixed/direct FIR, especially 11-tap FIR users | `rten-simd` | `simd/fixed-direct-fir.md` |

## Lower Priority Or Not Planned For SIMD First Pass

| Area | SIMD status | Reason |
| --- | --- | --- |
| biquad / IIR | not-planned for first pass | time-recursive dependency makes lane use nontrivial |
| compand envelope | not-planned for first pass | envelope, lookahead, and delay state dominate |
| dither / noise shaping | not-planned for first pass | PRNG and feedback state weaken simple vectorization |
| chorus / flanger / phaser / reverb | not-planned for first pass | delay-line layout and algorithm quality should be fixed first |
| contrast / tanh saturation | blocked | needs vector math or an explicit approximation-error policy |
| FFT internals | not-planned | delegated to FFT libraries unless profiling proves a pointwise bottleneck |

Create new SIMD detail files from `doc/development/templates/simd-kernel.md`.
