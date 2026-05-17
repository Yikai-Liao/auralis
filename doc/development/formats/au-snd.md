---
kind: format
format: "au-snd"
status: planned
owner: "auralis-codec"
backend: "Symphonia decode"
decode: planned
encode: not-planned
---

# AU/SND Format Boundary

## Scope

AU/SND support should be a thin decode path inside `auralis-codec`, backed by
Symphonia when the format is in scope.

## Adapter Rule

AU/SND is a codec facade entry, not a separate target crate. It must not
introduce native codec dependencies, graph-specific validation, local codec
implementations, or effect behavior.

## Sample Representation

AU/SND multi-byte samples are big-endian. Decode normalizes supported
Symphonia output into planar `f32`; export stays out of scope until an encoder
backend is deliberately selected.

## Validation

- validate magic, header, data offset, encoding code, sample rate, and channel
  count;
- reject unsupported encoding codes;
- handle known and unknown data size according to documented policy;
- keep encode options absent until encode is deliberately planned.

## Tests

- u-law, A-law, integer PCM, and float decode fixtures where Symphonia supports
  them;
- invalid header diagnostics;
- big-endian sample fixtures;
- deterministic decode checks.

## Benchmarks

Benchmark decode separately from graph execution. AU/SND does not require
SoX-ng as an implementation dependency.

## Done When

AU/SND is covered by the `auralis-codec` facade and all unsupported encodings
fail with typed diagnostics.
