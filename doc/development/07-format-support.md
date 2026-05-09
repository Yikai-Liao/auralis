# 7.x Format Support Milestone

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

## Milestone 7.0: codec backend policy

### Feature 7.0.1: pure Rust codec backend policy

Status: planned.

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

### Feature 7.0.2: encoder trait and output format model

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
| WAV | `hound` behind Auralis adapter | implemented / stage 1 | default output format |
| RAW PCM | Auralis-owned sample layout and endian conversion | planned | small boundary; not a complex codec |
| AIFF / AIFC | pure Rust crate candidate, such as `aifc`, after audit | planned | feature-gated adapter |
| FLAC | pure Rust encoder candidate, such as `flacenc`, after audit | planned experimental | no libFLAC wrapper under current policy |
| MP3 | pure Rust encoder only if a credible backend is selected | not planned now | no LAME wrapper |
| Ogg Vorbis | pure Rust encoder only if a credible backend is selected | not planned now | no libvorbis wrapper |
| Ogg Opus | pure Rust encoder and Ogg muxing only if credible backends are selected | not planned now | no libopusenc wrapper |
| AAC / M4A | pure Rust encoder and MP4 muxing only if credible backends are selected | not planned now | no `ffmpeg` or FDK-AAC |
| ALAC / MP4 | pure Rust encoder and muxing only if credible backends are selected | not planned now | no `ffmpeg` |
| WavPack | pure Rust backend only if selected later | future / not selected | not first-stage scope |

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

## Milestone 7.1: richer WAV support

### Feature 7.1.1: WAV PCM8

### Feature 7.1.2: WAV PCM24

### Feature 7.1.3: WAV PCM32

### Feature 7.1.4: WAV float32

### Feature 7.1.5: WAV float64

### Feature 7.1.6: WAV u-law and A-law

### Feature 7.1.7: WAV RIFX

## Milestone 7.2: raw formats

### Feature 7.2.1: raw signed and unsigned PCM

### Feature 7.2.2: raw float32 and float64

### Feature 7.2.3: raw endian, bit-order, and nibble-order options

## Milestone 7.3: AIFF formats

### Feature 7.3.1: AIFF PCM

Use a pure Rust AIFF backend candidate only after Feature 7.0.1 records the
dependency audit.

### Feature 7.3.2: AIFC encodings

Use a pure Rust AIFC backend candidate only after Feature 7.0.1 records the
dependency audit.

## Milestone 7.4: FLAC

### Feature 7.4.1: FLAC decode

Select a pure Rust FLAC decoder backend. Native libFLAC wrappers are not planned
under the current policy.

### Feature 7.4.2: FLAC encode

Select a pure Rust FLAC encoder backend. `flacenc` is a candidate to audit, but
its API stability, maintenance, correctness coverage, and performance must be
recorded before implementation. Native libFLAC wrappers are not planned under
the current policy.

## Milestone 7.5: AU/SND

### Feature 7.5.1: AU/SND

Use a pure Rust AU/SND backend or a small Auralis-owned PCM container adapter
only after the scope is documented.

## Milestone 7.6: external and native codec backends

### Feature 7.6.1: external `ffmpeg` backend

Status: not planned.

External `ffmpeg` decode or encode backends are outside the current pure Rust
policy.

### Feature 7.6.2: native codec wrapper backends

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
