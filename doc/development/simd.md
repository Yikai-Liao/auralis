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

This table is intentionally incomplete until the full migration commit moves
the SIMD roadmap into flat kernel documents.
