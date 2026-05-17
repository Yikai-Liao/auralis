---
kind: format
format: "unsupported-native-codecs"
status: not-planned
owner: "codec boundary"
backend: "none"
decode: not-planned
encode: not-planned
---

# Unsupported Codec Shortcuts

## Scope

This document records codec backend families and architecture shortcuts that
are not planned under the current `auralis-codec` facade policy.

## Not Planned

- new per-format target crates such as `auralis-flac`, `auralis-aiff`, or
  `auralis-au`;
- local implementations of complex codecs when Symphonia can provide decode;
- using `hound` as the primary WAV decode architecture instead of a fallback;
- adding non-WAV encode paths before an explicit encoder policy exists, except
  the planned FLAC encode path through `flacenc`;
- external `ffmpeg` command backends;
- `ffmpeg-next` or other FFmpeg link-time wrappers;
- libFLAC wrappers;
- LAME wrappers;
- libvorbis wrappers;
- libopusenc wrappers;
- FDK-AAC wrappers;
- other native codec libraries that would add system-library, ABI, licensing,
  or deployment drift.

## Adapter Rule

If MP3, Ogg Vorbis, Ogg Opus, AAC/M4A, ALAC/MP4, WavPack, or encode support
for currently decode-only formats is reconsidered, the new plan must first
select a credible encoder backend and keep it behind `auralis-codec`. It must
still remain an adapter over Auralis-owned buffers, diagnostics, and graph
outputs.

## Validation

Unsupported formats should fail with typed unsupported-format diagnostics. They
must not shell out to external tools from normal decode/encode paths.

## Tests

Keep unsupported-format diagnostics stable. Add fixtures only when a format is
moved out of `not-planned`.

## Done When

Codec shortcuts remain closed, `auralis-codec` stays the facade, and format
support decisions stay in short format documents instead of command docs or
README prose.
