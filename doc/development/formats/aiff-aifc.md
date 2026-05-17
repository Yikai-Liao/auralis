---
kind: format
format: "aiff-aifc"
status: implemented
owner: "auralis-aiff"
backend: "pure Rust aifc adapter behind Auralis-owned types"
decode: implemented
encode: implemented
---

# AIFF And AIFC Format Boundary

## Scope

AIFF/AIFC support covers plain AIFF signed-integer PCM and selected AIFC
encodings behind Auralis-owned adapter types.

## Adapter Rule

The `aifc` crate is a backend detail. Public API and graph behavior must use
Auralis-owned options, diagnostics, and buffer types.

## Sample Representation

Current support includes signed integer PCM, little-endian AIFC integer,
float32/float64, and G.711 u-law/A-law where implemented. Decode normalizes
into planar `f32`; export writes through explicit encode options.

## Validation

- reject unsupported compression IDs;
- validate bit depth and channel metadata;
- preserve deterministic handling for metadata and marker chunks that Auralis
  intentionally ignores or passes through;
- reject non-finite float output according to the format policy.

## Tests

- decode/export fixtures for supported AIFF and AIFC variants;
- unsupported compression diagnostics;
- metadata behavior fixtures where metadata is in scope;
- effect pipeline smoke only after standalone codec tests pass.

## Benchmarks

Benchmark decode and encode separately. Do not use native codec wrappers.

## Done When

AIFF/AIFC remains a pure Rust adapter and backend crate types do not leak into
public graph or library contracts.
