---
kind: format
format: "flac"
status: planned
owner: "auralis-codec"
backend: "Symphonia decode"
decode: planned
encode: not-planned
---

# FLAC Format Boundary

## Scope

FLAC support should be a thin `auralis-codec` decode path backed by Symphonia.
Auralis should not maintain a separate `auralis-flac` implementation as the
target architecture.

## Adapter Rule

Symphonia is the backend detail. Auralis owns public options and diagnostics.
FLAC encode is not planned until a concrete encoder policy is selected; do not
add a custom Auralis FLAC encoder or keep `claxon`/`flacenc` as the development
target.

## Sample Representation

Decode normalizes integer samples by the same convention used by WAV integer
PCM. Unsupported stream properties should surface as typed codec diagnostics.

## Validation

- reject unsupported FLAC stream properties with typed errors;
- keep encode options absent until encode is deliberately planned;
- ensure non-finite samples cannot silently enter integer encode.

## Tests

- tiny deterministic FLAC fixture decode;
- unsupported stream diagnostics;
- no dependency on external `flac`, `ffmpeg`, `claxon`, or `flacenc` paths.

## Benchmarks

Benchmark decode separately from graph execution, with fresh output directories
and no write to `target/benchmarks/sox_ng`.

## Done When

FLAC is covered by the `auralis-codec` facade through Symphonia decode, with no
local FLAC implementation, native wrapper dependency, or format-specific graph
semantics.
