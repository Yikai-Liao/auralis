---
kind: format
format: "unsupported-native-codecs"
status: not-planned
owner: "codec boundary"
backend: "none"
decode: not-planned
encode: not-planned
---

# Unsupported Native Codec Wrappers

## Scope

This document records codec backend families that are not planned under the
current pure Rust policy.

## Not Planned

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

If MP3, Ogg Vorbis, Ogg Opus, AAC/M4A, ALAC/MP4, or WavPack is reconsidered,
the new plan must first select a credible pure Rust backend or explicitly
change the policy. It must still remain an adapter over Auralis-owned buffers,
diagnostics, and graph outputs.

## Validation

Unsupported formats should fail with typed unsupported-format diagnostics. They
must not shell out to external tools from normal decode/encode paths.

## Tests

Keep unsupported-format diagnostics stable. Add fixtures only when a format is
moved out of `not-planned`.

## Done When

Native-wrapper shortcuts remain closed and format support decisions stay in
short format documents instead of command docs or README prose.
