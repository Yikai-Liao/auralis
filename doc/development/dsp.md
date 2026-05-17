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

This table is intentionally incomplete until the full migration commit moves
the existing DSP primitive roadmap into flat primitive documents.
