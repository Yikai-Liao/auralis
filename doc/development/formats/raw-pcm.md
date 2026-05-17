---
kind: format
format: "raw-pcm"
status: implemented
owner: "auralis-raw"
backend: "Auralis-owned raw PCM adapter"
decode: implemented
encode: implemented
---

# Raw PCM Format Boundary

## Scope

Raw PCM is an explicit sample-layout boundary with caller-provided metadata
such as sample format, byte order, bit order, nibble order, sample rate, and
channel count.

## Adapter Rule

Raw PCM must remain an adapter around Auralis buffers and `OutputFormat`.
Because raw files carry little or no metadata, graph or CLI callers must provide
the missing format facts before decode or encode.

## Sample Representation

Supported families include signed/unsigned integer PCM and IEEE float PCM where
the current adapter implements them. Decode normalizes into planar `f32`;
encode serializes from materialized Auralis buffers.

## Validation

- require explicit sample format, sample rate, and channel count;
- validate byte/bit/nibble order options;
- reject impossible frame sizes;
- keep unsupported raw encodings as typed errors.

## Tests

- signed and unsigned integer fixtures;
- float fixtures;
- byte-order and bit-order fixtures;
- invalid metadata diagnostics;
- deterministic encode byte fixtures.

## Benchmarks

Raw conversion benchmarks should separate sample conversion cost from file I/O
where possible.

## Done When

Raw PCM stays a small Auralis-owned boundary and does not become a second graph
or parameter language.
