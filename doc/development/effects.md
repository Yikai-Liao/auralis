---
kind: index
status: active
---

# Effects Index

This index tracks effect development documents. It is a navigation and status
file only. Put formulas, pseudocode, reference rationale, tests, and performance
notes in `doc/development/effects/<effect>.md`.

## Columns

| Column | Meaning |
| --- | --- |
| Effect | graph op / CLI `--fx` name |
| Status | `planned`, `active`, `implemented`, `blocked`, `not-planned`, or `superseded` |
| Reference target | selected oracle or algorithm reference |
| SoX-ng role | oracle, baseline, rejected, or not applicable |
| Priority | migration or implementation priority |
| Detail | link to the effect document |

## Migration Table

| Effect | Status | Reference target | SoX-ng role | Priority | Detail |
| --- | --- | --- | --- | --- | --- |
| `phaser` | planned | Faust `phaser2_*` | rejected for default; compat only | P1 | `effects/phaser.md` |
| `stretch` | planned | Signalsmith Stretch; PaulStretch for extreme stretch | rejected for default; compat only | P1 | `effects/stretch.md` |
| `noiseprof` | planned | stationary spectral profile model | legacy profile reference only | P1 | `effects/noiseprof.md` |
| `noisered` | planned | noisereduce-style stationary spectral gating | rejected for default; compat only | P1 | `effects/noisered.md` |
| `vad` | planned | WebRTC VAD wrapped in Auralis trim/padding semantics | rejected for default; compat only | P1 | `effects/vad.md` |
| `tempo` | planned | Signalsmith Stretch; Rubber Band as quality reference | baseline and compat only | P2 | migration target: `effects/tempo.md` |
| `pitch` | planned | Signalsmith Stretch; Rubber Band as quality reference | baseline and compat only | P2 | migration target: `effects/pitch.md` |
| `bend` | planned | Signalsmith Stretch; Rubber Band as quality reference | baseline and compat only | P2 | migration target: `effects/bend.md` |
| `chorus` | planned | DaisySP Chorus and Auralis multi-voice fractional delay | parameter compatibility only | P2 | migration target: `effects/chorus.md` |
| `flanger` | planned | Faust `flanger_*` and DaisySP Flanger | parameter compatibility only | P2 | migration target: `effects/flanger.md` |
| `reverb` | planned | Faust reverbs; FDN/Moorer/Schroeder family | Freeverb compat baseline only | P2 | migration target: `effects/reverb.md` |
| `overdrive` | planned | DaisySP Overdrive, MusicDSP waveshapers | baseline only | P3 | migration target: `effects/overdrive.md` |
| `saturation` | planned | Auralis-defined curves with MusicDSP/Airwindows references | baseline only | P3 | migration target: `effects/saturation.md` |
| `contrast` | planned | Auralis-defined nonlinear contrast curve | baseline only | P3 | migration target: `effects/contrast.md` |
| `compand` | planned | SoX-compatible curve processor; Faust for modern compressor track | compat target for SoX semantics | P3 | migration target: `effects/compand.md` |
| `mcompand` | planned | SoX-compatible multiband compander; Faust for modern compressor track | compat target for SoX semantics | P3 | migration target: `effects/mcompand.md` |

Rows marked `migration target` preserve prompt scope before their flat detail
files are written. Create each detail file from
`doc/development/templates/effect.md` when migrating that effect.
