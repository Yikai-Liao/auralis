---
kind: format
format: "flac"
status: planned
owner: "auralis-codec"
backend: "Symphonia decode, flacenc encode"
decode: planned
encode: planned
---

# FLAC Format Boundary

## Scope

FLAC support should be a thin `auralis-codec` boundary. Decode is backed by
Symphonia. Encode may be planned through `flacenc`, still hidden behind
`auralis-codec`. Auralis should not maintain a separate `auralis-flac`
implementation as the target architecture.

## Adapter Rule

Symphonia is the decode backend detail, and `flacenc` is the planned encoder
candidate. Auralis owns public options, diagnostics, and encode summaries. Do
not add a custom Auralis FLAC encoder or use `claxon` as the development
target.

## Sample Representation

Decode normalizes integer samples by the same convention used by WAV integer
PCM. Unsupported stream properties should surface as typed codec diagnostics.

## Validation

- reject unsupported FLAC stream properties with typed errors;
- keep encode options small and explicit while `flacenc` policy is developed;
- ensure non-finite samples cannot silently enter integer encode.

## Tests

- tiny deterministic FLAC fixture decode;
- unsupported stream diagnostics;
- no dependency on external `flac`, `ffmpeg`, or `claxon` paths;
- encode fixtures once the `flacenc` path is activated.

## Benchmarks

Benchmark decode separately from graph execution, with fresh output directories
and no write to `target/benchmarks/sox_ng`.

## Done When

FLAC is covered by the `auralis-codec` facade through Symphonia decode and a
planned `flacenc` encode backend, with no local FLAC implementation, native
wrapper dependency, or format-specific graph semantics.
