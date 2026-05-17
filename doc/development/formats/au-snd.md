---
kind: format
format: "au-snd"
status: implemented
owner: "auralis-au"
backend: "Auralis-owned AU/SND container adapter"
decode: implemented
encode: implemented
---

# AU/SND Format Boundary

## Scope

AU/SND support handles `.snd` streams for linear PCM, IEEE float, u-law, and
A-law encodings where implemented.

## Adapter Rule

AU/SND is a compact Auralis-owned container adapter. It must not introduce
native codec dependencies, graph-specific validation, or effect behavior.

## Sample Representation

AU/SND multi-byte samples are big-endian. Decode normalizes supported samples
into planar `f32`; export serializes from Auralis buffers through explicit
format options.

## Validation

- validate magic, header, data offset, encoding code, sample rate, and channel
  count;
- reject unsupported encoding codes;
- handle known and unknown data size according to documented policy;
- reject or report non-finite samples for integer encode.

## Tests

- u-law, A-law, integer PCM, and float fixtures;
- invalid header diagnostics;
- big-endian sample fixtures;
- encode/decode deterministic checks.

## Benchmarks

Benchmark decode and encode separately. AU/SND does not require SoX-ng as an
implementation dependency.

## Done When

AU/SND remains an in-tree adapter and all unsupported encodings fail with typed
diagnostics.
