---
kind: format
format: "flac"
status: implemented
owner: "auralis-flac"
backend: "pure Rust claxon/flacenc adapters behind Auralis-owned types"
decode: implemented
encode: implemented
---

# FLAC Format Boundary

## Scope

FLAC support decodes integer FLAC streams into planar `f32` buffers and exports
deterministic PCM16 FLAC through `OutputFormat::Flac`.

## Adapter Rule

`claxon` and `flacenc` are backend details. Auralis owns public options,
diagnostics, and encode summaries. Native libFLAC wrappers are not planned
under the current policy.

## Sample Representation

Decode normalizes integer samples by the same convention used by WAV integer
PCM. Encode currently targets a conservative deterministic PCM16 FLAC profile
unless a later format document expands controls.

## Validation

- reject unsupported FLAC stream properties with typed errors;
- reject unsupported encode options;
- keep compression-level choices explicit if they are added later;
- ensure non-finite samples cannot silently enter integer encode.

## Tests

- tiny deterministic FLAC fixture decode;
- encode fixture and decode-back validation;
- unsupported stream diagnostics;
- no dependency on external `flac` or `ffmpeg` commands.

## Benchmarks

Benchmark decode and encode separately, with fresh output directories and no
write to `target/benchmarks/sox_ng`.

## Done When

FLAC remains a pure Rust adapter with no native wrapper dependency and no
format-specific graph semantics.
