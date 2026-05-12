# 8.x Format Support Milestone

Additional formats are intentionally after effect and pipeline coverage. Add a
format earlier only when it is required to test an effect that cannot be tested
faithfully through WAV PCM16.

Current policy: only pure Rust codec backends are planned. Auralis should not
shell out to `ffmpeg`, link `ffmpeg-next`, or bind native codec libraries as the
format roadmap's default answer. Reconsidering that rule requires an explicit
development-plan change, not a leaf-feature implementation detail.

Auralis owns:

```text
AudioBuffer -> codec traits -> pure Rust backend adapter
```

It does not own:

```text
custom FLAC / MP3 / AAC / Vorbis / Opus encoder algorithms
external ffmpeg command integration
native libFLAC / LAME / libopusenc / FDK-AAC wrappers
```

Each format leaf feature uses the shared acceptance tests below.

## Milestone 8.0: codec backend policy

### Feature 8.0.1: pure Rust codec backend policy

Status: completed.

Document the decode/import and encode/export backend policy before adding any
new format dependency.

Acceptance tests:

- README documents that future codec backends must be pure Rust unless a later
  policy change explicitly says otherwise;
- the selected backend for each planned format is classified as built-in,
  feature-gated pure Rust, experimental pure Rust, or not planned;
- every candidate dependency has an audit note for license, maintenance,
  transitive native dependencies, streaming behavior, supported sample formats,
  metadata behavior, and fuzz/security risk;
- no backend crate type leaks into `auralis-core` or public high-level APIs;
- no new lossy encoder dependency is added by this policy feature.

Implementation notes:

- README now states that future codec backends stay pure Rust by default and
  that additional formats remain blocked until their backend audit is recorded.
- This roadmap now classifies the currently selected backend direction for each
  planned or intentionally unplanned format family using the four buckets
  required by this feature: built-in, feature-gated pure Rust, experimental
  pure Rust, and not planned.
- The backend audit notes below are policy records only. They do not add any
  new dependencies, and they explicitly keep backend crate details behind
  Auralis-owned adapter modules instead of exposing them through public APIs.

### Feature 8.0.1 backend audit

| Format family | Candidate backend | Classification | License and maintenance note | Native dependency note | Streaming and sample-format note | Metadata and fuzz/security note |
|---|---|---|---|---|---|---|
| WAV | `hound` through `auralis-wav` / `auralis-codec` adapters | built-in | Existing dependency with a stable Rust ecosystem footprint and already exercised by the current WAV path. | Pure Rust crate; no external codec libraries or system tools. | Current stage remains PCM16-only at the Auralis boundary; richer WAV sample types stay as later leaf features. | RIFF/WAV metadata stays intentionally narrow for now; existing parser coverage, fuzz seeds, and unsupported-format diagnostics remain the safety baseline. |
| RAW PCM | Auralis-owned reader/writer and endian/layout conversion | built-in | No third-party codec crate is required because raw PCM is a container-less boundary owned by Auralis. | No native dependencies. | Streaming-friendly because bytes map directly to frames; future leaves must define endian, signedness, float, and nibble/bit-order handling explicitly. | Metadata is intentionally minimal by design, so the main risk is option parsing and shape validation rather than tag handling; parser/fuzz coverage should focus there. |
| AIFF / AIFC | pure Rust crate candidate such as `aifc`, behind a feature-gated adapter | feature-gated pure Rust | Accept only after a crate-level audit confirms MIT/Apache-compatible licensing, active enough maintenance, and reviewable transitive footprint. | Must remain pure Rust with no libsndfile or other native wrapper path. | Needs a streaming decode/encode story for PCM AIFF first; compressed AIFC encodings are later and may stay narrower if the backend cannot cover them safely. | Chunk metadata behavior must stay behind Auralis-owned option/report types; add parser/fuzz coverage before enabling broad import/export support. |
| FLAC | pure Rust crate candidate such as `flacenc` plus a pure Rust decoder candidate, both behind adapters | experimental pure Rust | Candidate crates are acceptable only after a feature-level audit records license compatibility, maintenance health, and any 0.x stability caveats. | No `libFLAC`, `ffmpeg`, or other native wrapper path is allowed under the current policy. | The backend must document streaming encode/decode limits, supported bit depths, and channel/sample-rate constraints before the FLAC leaves can land. | FLAC metadata blocks, framing validation, and malformed-stream handling need explicit review and fuzz coverage because they expand the parser attack surface beyond WAV. |
| MP3 | none selected | not planned | No credible pure Rust encoder is selected today, and adding one is outside the current roadmap. | Native-backed paths such as LAME are outside policy. | Lossy psychoacoustic streaming complexity is intentionally out of scope for the initial format roadmap. | Security and metadata considerations are deferred because no backend is being considered in this phase. |
| Ogg Vorbis | none selected | not planned | No credible pure Rust encoder/muxer combination is selected today. | libvorbis and other native wrapper paths are outside policy. | Streaming container plus codec complexity is out of scope until a future policy change or strong pure Rust backend appears. | Ogg page parsing and Vorbis comment handling are deferred with the format itself. |
| Ogg Opus | none selected | not planned | No credible pure Rust Opus encoder plus Ogg muxing path is selected today. | `libopusenc` and wrapper paths are outside policy. | Real support would require both codec and container decisions, which are intentionally postponed. | Ogg/Opus parser and metadata risks are deferred with the format itself. |
| AAC / M4A | none selected | not planned | No credible pure Rust AAC encoder plus MP4 muxing path is selected today. | FDK-AAC, FFmpeg, and similar native-backed solutions are outside policy. | Lossy codec plus MP4 container work is explicitly outside first-stage scope. | MP4 atom parsing and metadata handling are deferred with the format itself. |
| ALAC / MP4 | none selected | not planned | No credible pure Rust ALAC plus MP4 muxing path is selected today. | FFmpeg and native wrapper paths are outside policy. | Container plus codec scope is deferred until far after core WAV/PCM/AIFF/FLAC work. | MP4 parser and metadata risks are deferred with the format itself. |
| WavPack | none selected | not planned | No backend has been selected, and it is not part of the first-stage roadmap. | Native-backed shortcuts remain outside policy even if a wrapper exists later. | Streaming and hybrid-lossy mode semantics would need their own audit before any selection. | Container/metadata/fuzz considerations are postponed until a future selection exists. |

### Feature 8.0.2: encoder trait and output format model

Status: planned.

Define Auralis-owned encode abstractions before adding more encoders.

Suggested public shape:

```rust
pub enum OutputFormat {
    Wav(WavEncodeOptions),
    RawPcm(RawPcmEncodeOptions),
    Aiff(AiffEncodeOptions),
    Flac(FlacEncodeOptions),
}

pub trait AudioEncoder {
    fn encode(
        &self,
        input: &AudioBuffer,
        writer: &mut dyn std::io::Write,
    ) -> Result<EncodeSummary>;
}
```

Acceptance tests:

- public APIs are expressed in Auralis-owned option types;
- backend-specific crate types remain private to adapter modules or crates;
- unsupported output formats fail with typed errors;
- WAV behavior remains unchanged.

## Pure Rust encode/export roadmap

| Output format | Backend policy | Status | Notes |
|---|---|---|---|
| WAV | `hound` behind Auralis adapter | built-in | default output format |
| RAW PCM | Auralis-owned sample layout and endian conversion | built-in | small boundary; not a complex codec |
| AIFF / AIFC | pure Rust crate candidate, such as `aifc`, after audit | feature-gated pure Rust | adapter stays optional until the crate audit is accepted |
| FLAC | pure Rust encoder candidate, such as `flacenc`, after audit | experimental pure Rust | no libFLAC wrapper under current policy |
| MP3 | pure Rust encoder only if a credible backend is selected | not planned | no LAME wrapper |
| Ogg Vorbis | pure Rust encoder only if a credible backend is selected | not planned | no libvorbis wrapper |
| Ogg Opus | pure Rust encoder and Ogg muxing only if credible backends are selected | not planned | no libopusenc wrapper |
| AAC / M4A | pure Rust encoder and MP4 muxing only if credible backends are selected | not planned | no `ffmpeg` or FDK-AAC |
| ALAC / MP4 | pure Rust encoder and muxing only if credible backends are selected | not planned | no `ffmpeg` |
| WavPack | pure Rust backend only if selected later | not planned | not first-stage scope |

The exact crate and version are selected during the leaf feature with
`cargo add` / `cargo check` and a dependency audit. A 0.x pure Rust crate may be
accepted only behind an Auralis-owned adapter and feature gate; its types must
not become public API.

## Not planned under the pure Rust policy

The following backend families are not planned in the current roadmap:

- external `ffmpeg` command backends;
- `ffmpeg-next` or other FFmpeg link-time wrappers;
- libFLAC wrappers such as a production `flac-encoder` backend;
- LAME wrappers such as `mp3lame-encoder`;
- libvorbis wrappers;
- `libopusenc` wrappers;
- FDK-AAC wrappers;
- any backend that requires system codec libraries at runtime or build time.

## Milestone 8.1: richer WAV support

### Feature 8.1.1: WAV PCM8

### Feature 8.1.2: WAV PCM24

### Feature 8.1.3: WAV PCM32

### Feature 8.1.4: WAV float32

### Feature 8.1.5: WAV float64

### Feature 8.1.6: WAV u-law and A-law

### Feature 8.1.7: WAV RIFX

## Milestone 8.2: raw formats

### Feature 8.2.1: raw signed and unsigned PCM

### Feature 8.2.2: raw float32 and float64

### Feature 8.2.3: raw endian, bit-order, and nibble-order options

## Milestone 8.3: AIFF formats

### Feature 8.3.1: AIFF PCM

Use a pure Rust AIFF backend candidate only after Feature 8.0.1 records the
dependency audit.

### Feature 8.3.2: AIFC encodings

Use a pure Rust AIFC backend candidate only after Feature 8.0.1 records the
dependency audit.

## Milestone 8.4: FLAC

### Feature 8.4.1: FLAC decode

Select a pure Rust FLAC decoder backend. Native libFLAC wrappers are not planned
under the current policy.

### Feature 8.4.2: FLAC encode

Select a pure Rust FLAC encoder backend. `flacenc` is a candidate to audit, but
its API stability, maintenance, correctness coverage, and performance must be
recorded before implementation. Native libFLAC wrappers are not planned under
the current policy.

## Milestone 8.5: AU/SND

### Feature 8.5.1: AU/SND

Use a pure Rust AU/SND backend or a small Auralis-owned PCM container adapter
only after the scope is documented.

## Milestone 8.6: external and native codec backends

### Feature 8.6.1: external `ffmpeg` backend

Status: not planned.

External `ffmpeg` decode or encode backends are outside the current pure Rust
policy.

### Feature 8.6.2: native codec wrapper backends

Status: not planned.

Native wrappers for libFLAC, LAME, libvorbis, libopusenc, FDK-AAC, FFmpeg, or
similar codec libraries are outside the current pure Rust policy.

## Format acceptance tests

Each format leaf feature must include:

- dependency audit proving the selected backend is pure Rust, or a documented
  not-planned status;
- decode and encode fixtures where the format supports both;
- unsupported encoding diagnostics;
- metadata preservation where the format has metadata;
- SoX-ng decode comparison into a common WAV or raw-float representation;
- effect pipeline tests using the new format only after standalone codec tests
  pass;
- proof that tests and runtime do not require external codec commands such as
  `ffmpeg`;
- clear public API boundaries so codec implementation crates do not leak into
  `auralis-core` or the high-level facade.
