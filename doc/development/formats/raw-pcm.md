---
kind: format
format: "raw-pcm"
status: planned
owner: "auralis-codec"
backend: "codec-internal raw PCM parser/writer"
decode: placeholder
encode: placeholder
---

# Raw PCM Format Boundary

## Scope

Raw PCM is a placeholder for an explicit sample-layout boundary inside
`auralis-codec` with caller-provided metadata such as sample format, byte
order, bit order, nibble order, sample rate, and channel count.

## Adapter Rule

Raw PCM must remain an adapter around Auralis buffers and explicit codec
options. Because raw files carry little or no metadata, graph or CLI callers
must provide the missing format facts before decode or encode. It must not
become a separate crate-level architecture parallel to `auralis-codec`.

## Sample Representation

Reserved families include signed/unsigned integer PCM and IEEE float PCM.
Decode should normalize into planar `f32`; encode remains a placeholder until
non-WAV encode policy is selected.

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
- deterministic byte fixtures when the placeholder is activated.

## Benchmarks

Raw conversion benchmarks should separate sample conversion cost from file I/O
where possible.

## Done When

Raw PCM stays a small `auralis-codec` placeholder and does not become a second
graph, parameter language, or self-contained codec crate.
