---
kind: format
format: "wav"
status: implemented
owner: "auralis-codec"
backend: "Symphonia first, hound fallback only when needed"
decode: implemented
encode: implemented
---

# WAV Format Boundary

## Scope

WAV is the built-in default audio container boundary. The target architecture
keeps the public boundary in `auralis-codec`: decode uses Symphonia first for
every WAV file. `hound` is only the fallback or special-case WAV path when
Symphonia explicitly cannot handle an Auralis-supported requirement. Encode
remains an Auralis-owned boundary and may use `hound` internally.

## Adapter Rule

WAV is a codec/container adapter only. It must not own graph validation, graph
semantics, effect parameter parsing, or execution behavior. CLI and graph
frontends lower to `GraphRequest`; `auralis-codec` only materializes audio
sources and sinks.

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
- fallback selection fixtures proving Symphonia is attempted first and `hound`
  is used only for the documented fallback cases;
- effect pipeline tests only after standalone codec coverage passes.

## Benchmarks

WAV decode/encode benchmarks should use fresh output directories and must not
write to `target/benchmarks/sox_ng`.

## Done When

WAV remains the default built-in `auralis-codec` adapter, backend crate types do
not leak out, and no WAV-specific behavior leaks into graph validation or
execution.
