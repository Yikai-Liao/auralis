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
| WAV | implemented | `auralis-wav` | Auralis-owned | implemented | implemented | `formats/wav.md` |
| Raw PCM | implemented | `auralis-raw` | Auralis-owned | implemented | implemented | `formats/raw-pcm.md` |
| AIFF/AIFC | implemented | `auralis-aiff` | pure Rust adapter | implemented | implemented | `formats/aiff-aifc.md` |
| FLAC | implemented | `auralis-flac` | pure Rust adapter | implemented | implemented | `formats/flac.md` |
| AU/SND | implemented | `auralis-au` | Auralis-owned | implemented | implemented | `formats/au-snd.md` |
| Native codec wrappers | not-planned | codec boundary | none | not-planned | not-planned | `formats/unsupported-native-codecs.md` |

Create new format documents as short adapter-boundary files under
`doc/development/formats/`.
