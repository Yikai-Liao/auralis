---
kind: index
status: active
---

# Format And Codec Index

This index tracks format and codec boundary documents. Keep format semantics,
sample representation, validation rules, tests, and benchmarks in
`doc/development/formats/<format>.md`.

## Columns

| Column | Meaning |
| --- | --- |
| Format | container or raw audio boundary |
| Status | shared status value |
| Owner | crate/module responsible for the boundary |
| Backend | pure Rust adapter or internal implementation |
| Decode | decode support status |
| Encode | encode support status |
| Detail | link to the format document |

## Migration Table

| Format | Status | Owner | Backend | Decode | Encode | Detail |
| --- | --- | --- | --- | --- | --- | --- |
| WAV | planned | `auralis-wav` | Auralis-owned | implemented | implemented | `formats/wav.md` |
| FLAC | planned | `auralis-flac` | pure Rust adapter | implemented | implemented | `formats/flac.md` |

This table is intentionally incomplete until the full migration commit moves
the format roadmap into flat format documents.
