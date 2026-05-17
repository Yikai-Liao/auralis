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
| `phaser` | planned | Faust `phaser2_*` | rejected | P1 | `effects/phaser.md` |
| `stretch` | planned | Signalsmith Stretch | rejected | P1 | `effects/stretch.md` |
| `noisered` | planned | stationary spectral gating | rejected | P1 | `effects/noisered.md` |
| `vad` | planned | WebRTC VAD | rejected | P1 | `effects/vad.md` |

This table is intentionally incomplete until the full migration commit moves
all existing effect plans out of the numbered roadmap files.
