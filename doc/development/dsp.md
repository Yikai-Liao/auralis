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
| `axpy` | planned | `auralis-dsp` | mix, remix, wet/dry | planned | `dsp/axpy.md` |
| `scale-into` | planned | `auralis-dsp` | gain-style transforms, normalization, remix | planned | `dsp/scale-into.md` |
| `mul-by-slice` | planned | `auralis-dsp` | tremolo, modulation, envelope apply | planned | `dsp/mul-by-slice.md` |
| `clamp` | planned | `auralis-dsp` | output policies, remix, nonlinear effects | planned | `dsp/clamp.md` |
| `finite-peak-rms` | planned | `auralis-dsp` | stats, normalize, write validation | planned | `dsp/finite-peak-rms.md` |
| `fractional-delay` | planned | `auralis-dsp` | chorus, flanger, phaser, future vibrato | not applicable first pass | `dsp/fractional-delay.md` |
| `stft-profile` | planned | `auralis-dsp` | noiseprof, noisered, denoise experiments | not applicable first pass | `dsp/stft-profile.md` |
| `fixed-direct-fir` | planned | `auralis-dsp` | fir, earwax, loudness, sinc/hilbert support | planned | `dsp/fixed-direct-fir.md` |

Create new DSP primitive documents from
`doc/development/templates/dsp-primitive.md`.
