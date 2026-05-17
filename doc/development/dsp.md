---
kind: index
status: active
---

# DSP Primitive Index

This index tracks reusable DSP primitive documents. It is a navigation and
status file only. Put formulas, API sketches, pseudocode, numerical contracts,
and performance notes in `doc/development/dsp/<primitive>.md`.

## Columns

| Column | Meaning |
| --- | --- |
| Primitive | reusable DSP kernel or state object |
| Status | shared status value |
| Owner | target crate/module |
| Callers | current or planned effect users |
| SIMD status | implemented, planned, not applicable, or blocked |
| Detail | link to the primitive document |

## Migration Table

| Primitive | Status | Owner | Callers | SIMD status | Detail |
| --- | --- | --- | --- | --- | --- |
| `axpy` | planned | `auralis-dsp` | mix, remix, wet/dry | planned | migration target: `dsp/axpy.md` |
| `scale-into` | planned | `auralis-dsp` | gain-style transforms, normalization, remix | planned | migration target: `dsp/scale-into.md` |
| `mul-by-slice` | planned | `auralis-dsp` | tremolo, modulation, envelope apply | planned | migration target: `dsp/mul-by-slice.md` |
| `clamp` | planned | `auralis-dsp` | output policies, remix, nonlinear effects | planned | migration target: `dsp/clamp.md` |
| `finite-peak-rms` | planned | `auralis-dsp` | stats, normalize, write validation | planned | migration target: `dsp/finite-peak-rms.md` |
| `fractional-delay` | planned | `auralis-dsp` | chorus, flanger, phaser, future vibrato | not applicable first pass | migration target: `dsp/fractional-delay.md` |
| `stft-profile` | planned | `auralis-dsp` | noiseprof, noisered, denoise experiments | not applicable first pass | migration target: `dsp/stft-profile.md` |
| `fixed-direct-fir` | planned | `auralis-dsp` | fir, earwax, loudness, sinc/hilbert support | planned | migration target: `dsp/fixed-direct-fir.md` |

Rows marked `migration target` preserve primitive scope before flat detail files
are written. Create each detail file from
`doc/development/templates/dsp-primitive.md` when migrating that primitive.
