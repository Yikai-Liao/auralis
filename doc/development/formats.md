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
| Backend | decoder or encoder backend wrapped by `auralis-codec` |
| Decode | decode support status |
| Encode | encode support status; `placeholder` means reserved for later policy |
| Detail | link to the format document |

## Migration Table

| Format | Status | Owner | Backend | Decode | Encode | Detail |
| --- | --- | --- | --- | --- | --- | --- |
| WAV | active | `auralis-codec` | Symphonia first, `hound` fallback only when needed | active | active | `formats/wav.md` |
| Raw PCM | planned | `auralis-codec` | codec-internal parser/writer | placeholder | placeholder | `formats/raw-pcm.md` |
| AIFF/AIFC | planned | `auralis-codec` | Symphonia decode only | planned | placeholder | `formats/aiff-aifc.md` |
| FLAC | planned | `auralis-codec` | Symphonia decode, `flacenc` encode | planned | planned | `formats/flac.md` |
| AU/SND | planned | `auralis-codec` | Symphonia decode only | planned | placeholder | `formats/au-snd.md` |
| Native codec wrappers | not-planned | codec boundary | none | not-planned | not-planned | `formats/unsupported-native-codecs.md` |

Create new format documents as short adapter-boundary files under
`doc/development/formats/`.
