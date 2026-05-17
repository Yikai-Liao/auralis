---
kind: historical-roadmap
status: superseded
superseded_by:
  - dsp.md
  - effects.md
  - simd.md
---

# 2.x Basic DSP Milestone

This milestone established the initial scalar DSP kernels, typed effects,
library facade, and CLI transforms.
It is retained for history and acceptance traceability; new DSP, effect, and
SIMD planning belongs in the short target documents.

## Feature 2.1: scalar `gain` kernel

Status: implemented.

Acceptance tests:

- `0 dB` identity;
- `-6 dB` and `+6 dB` analytical multipliers;
- empty, one-sample, odd-length, and full-scale inputs;
- finite input behavior is documented.

## Feature 2.2: `Gain` effect processor

Status: implemented.

Acceptance tests:

- effect output matches scalar kernel;
- whole-buffer and chunked-buffer processing match;
- typed config validation is documented.

## Feature 2.3: library chain API for gain

Status: implemented.

Acceptance tests:

- `AudioFile::open_wav(...).into_pipeline().gain_db(...).write_wav(...)`;
- output matches direct effect use;
- errors remain typed.

## Feature 2.4: CLI `gain`

Status: implemented.

Acceptance tests:

- `auralis render input.wav -o output.wav --fx 'gain -3'`;
- CLI output matches library output;
- SoX-ng golden comparison.

## Feature 2.5: `trim`

Status: implemented.

Acceptance tests:

- frame and seconds ranges;
- exact output length and frame boundaries;
- full-range identity;
- invalid ranges fail clearly;
- CLI and library behavior match;
- SoX-ng golden comparison.

## Feature 2.6: `pad`

Status: implemented.

Acceptance tests:

- start and end padding;
- zero padding identity;
- inserted samples are exact zeros;
- CLI and library behavior match;
- SoX-ng golden comparison.

## Feature 2.7: `reverse`

Status: implemented.

Acceptance tests:

- mono exact reverse;
- stereo frame-level reverse, not sample-level channel swap;
- reverse twice is identity;
- zero-length and one-frame buffers;
- CLI and library behavior match;
- SoX-ng golden comparison.

## Feature 2.8: `dcshift`

Status: implemented.

Acceptance tests:

- zero shift identity;
- positive and negative shifts;
- clipping behavior documented;
- no NaN for finite input;
- CLI and library behavior match;
- SoX-ng golden comparison if command semantics align.

## Feature 2.9: `fade`

Status: implemented.

Acceptance tests:

- envelope coefficients are correct;
- zero-duration fade identity or documented error;
- fade-in only;
- fade-out only;
- mono/stereo behavior;
- chunk invariance;
- CLI and library behavior match.

Known follow-up: standalone fade golden coverage is tracked in Feature 5.7.3.
