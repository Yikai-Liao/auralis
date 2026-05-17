---
kind: format
format: "wav"
status: implemented
owner: "auralis-wav"
backend: "Auralis-owned WAV adapter"
decode: implemented
encode: implemented
---

# WAV Format Boundary

## Scope

WAV is the built-in default audio container boundary. It decodes supported WAV
sample formats into Auralis planar `f32` buffers and encodes Auralis buffers
through `OutputFormat::Wav(WavEncodeOptions)`.

## Adapter Rule

WAV is a codec/container adapter only. It must not own graph validation, graph
semantics, effect parameter parsing, or execution behavior. CLI and graph
frontends lower to `GraphRequest`; WAV only materializes audio sources and
sinks.

## Sample Representation

Supported decode/export families:

- PCM8, PCM16, PCM24, PCM32;
- float32 and float64;
- u-law and A-law;
- little-endian RIFF and big-endian RIFX where implemented.

Internal representation is planar `f32`. Integer PCM normalization must remain
deterministic and match the documented full-scale denominators.

## Validation

- reject unsupported format tags with typed diagnostics;
- reject unsupported bit depths;
- validate channel count and sample rate before buffer construction;
- preserve non-finite handling policy for float writes.

## Tests

- decode and encode fixtures for each supported sample family;
- round trips where the format permits deterministic round trip;
- unsupported-format diagnostics;
- RIFX byte-order coverage;
- effect pipeline tests only after standalone codec coverage passes.

## Benchmarks

WAV decode/encode benchmarks should use fresh output directories and must not
write to `target/benchmarks/sox_ng`.

## Done When

WAV remains the default built-in adapter and no WAV-specific behavior leaks into
graph validation or execution.
