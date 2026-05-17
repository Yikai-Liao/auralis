---
kind: format
format: "aiff-aifc"
status: planned
owner: "auralis-codec"
backend: "Symphonia decode"
decode: planned
encode: not-planned
---

# AIFF And AIFC Format Boundary

## Scope

AIFF/AIFC support should be a thin decode path inside `auralis-codec`, using
Symphonia when the format is in scope. Auralis should not build and maintain a
separate `auralis-aiff` implementation as the target architecture.

## Adapter Rule

Symphonia is the backend detail. Public API and graph behavior must use
Auralis-owned options, diagnostics, and buffer types. Encode is out of scope
until a concrete encoder backend is selected; do not add a custom Auralis AIFF
encoder just to fill the matrix.

## Sample Representation

Planned decode support should normalize Symphonia-decoded samples into planar
`f32`. AIFC compression families that Symphonia does not support should fail
with typed unsupported-format diagnostics instead of growing local codecs.

## Validation

- reject unsupported compression IDs;
- validate bit depth and channel metadata;
- preserve deterministic handling for metadata and marker chunks that Auralis
  intentionally ignores or passes through;
- keep encode options absent until encode is deliberately planned.

## Tests

- decode fixtures for supported AIFF and AIFC variants;
- unsupported compression diagnostics;
- metadata behavior fixtures where metadata is in scope;
- effect pipeline smoke only after standalone codec tests pass.

## Benchmarks

Benchmark decode separately from graph execution. Do not add native wrappers or
local encoder work in this plan.

## Done When

AIFF/AIFC is covered by the `auralis-codec` facade, backend crate types do not
leak into public graph or library contracts, and encode remains unplanned.
