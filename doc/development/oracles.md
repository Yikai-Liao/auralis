---
kind: index
status: active
---

# Oracle And Reference Index

This index tracks shared oracle and reference-target decisions. Single-effect
reference decisions stay in that effect document unless the same policy applies
to multiple effects.

## Columns

| Column | Meaning |
| --- | --- |
| Subject | shared oracle topic |
| Status | shared status value |
| Reference target | selected modern reference or analytical oracle |
| Applies to | affected effects, formats, or primitives |
| Detail | link to the oracle note |

## Migration Table

| Subject | Status | Reference target | Applies to | Detail |
| --- | --- | --- | --- | --- |
| `time-pitch` | planned | Signalsmith Stretch, Rubber Band as quality reference | stretch, tempo, pitch, bend | `oracles/time-pitch.md` |
| `spectral-denoise` | planned | stationary spectral gating, RNNoise for speech-only future work | noiseprof, noisered, future speech denoise | `oracles/spectral-denoise.md` |

This table is intentionally incomplete until the full migration commit moves
shared reference policies out of legacy effect-roadmap text.
