---
kind: index
status: active
---

# Oracle And Reference Index

This index tracks shared oracle and reference-target decisions. Single-effect
reference decisions stay in that effect document unless the same policy applies
to multiple effects.

## Columns

| Column | Meaning |
| --- | --- |
| Subject | shared oracle topic |
| Status | shared status value |
| Reference target | selected modern reference or analytical oracle |
| Applies to | affected effects, formats, or primitives |
| Detail | link to the oracle note |

## Migration Table

| Subject | Status | Reference target | Applies to | Detail |
| --- | --- | --- | --- | --- |
| `time-pitch` | planned | Signalsmith Stretch; Rubber Band as quality reference | stretch, tempo, pitch, bend | migration target: `oracles/time-pitch.md` |
| `extreme-stretch` | planned | PaulStretch / PaulXStretch | future extreme stretch mode | migration target: `oracles/extreme-stretch.md` |
| `spectral-denoise` | planned | noisereduce-style stationary spectral gating | noiseprof, noisered | migration target: `oracles/spectral-denoise.md` |
| `speech-denoise` | planned | RNNoise and SpeexDSP | future speech-denoise effect; not `noisered` | migration target: `oracles/speech-denoise.md` |
| `voice-activity` | planned | WebRTC VAD with Auralis edit semantics | vad | migration target: `oracles/voice-activity.md` |
| `modulation-delay` | planned | Faust `phaflangers.lib`, DaisySP Chorus/Flanger | phaser, chorus, flanger, future vibrato | migration target: `oracles/modulation-delay.md` |
| `reverb` | planned | Faust reverbs; Freeverb for compat baseline | reverb and future room/ambience variants | migration target: `oracles/reverb.md` |
| `saturation-distortion` | planned | DaisySP Overdrive, MusicDSP formulas, Airwindows listening references | overdrive, saturation, contrast | migration target: `oracles/saturation-distortion.md` |
| `dynamics` | planned | Faust compressors for modern dynamics; SoX for compat curves | compand, mcompand, future compressor/limiter | migration target: `oracles/dynamics.md` |

Rows marked `migration target` preserve shared reference-policy scope before
their flat note files are written. Create each note from
`doc/development/templates/oracle-notes.md` when the corresponding effects are
migrated.
