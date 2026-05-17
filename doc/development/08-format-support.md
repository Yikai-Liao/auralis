---
kind: development-plan
status: superseded
superseded_by:
  - formats.md
  - formats/wav.md
  - formats/raw-pcm.md
  - formats/aiff-aifc.md
  - formats/flac.md
  - formats/au-snd.md
  - formats/unsupported-native-codecs.md
  - ../../docs/adr/0015-use-auralis-codec-over-symphonia.md
---

# Superseded: Format Support Milestone

The old 8.x per-format milestone has been migrated into the short format
documents and ADR 0015.

Authoritative files:

- [`formats.md`](formats.md) for the format and codec index;
- [`formats/wav.md`](formats/wav.md) for WAV decode and encode;
- [`formats/flac.md`](formats/flac.md) for Symphonia-backed FLAC decode and
  planned `flacenc` encode;
- [`formats/aiff-aifc.md`](formats/aiff-aifc.md) and
  [`formats/au-snd.md`](formats/au-snd.md) for Symphonia-backed decode-only
  planning;
- [`formats/raw-pcm.md`](formats/raw-pcm.md) for the raw PCM placeholder;
- [`formats/unsupported-native-codecs.md`](formats/unsupported-native-codecs.md)
  for rejected codec shortcuts;
- [`../../docs/adr/0015-use-auralis-codec-over-symphonia.md`](../../docs/adr/0015-use-auralis-codec-over-symphonia.md)
  for the codec facade decision.

Do not add new format planning here. The target architecture is `auralis-codec`
as the facade over Symphonia decode, WAV encode first, planned FLAC encode via
`flacenc`, and placeholders for other encode paths.
