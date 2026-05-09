# 1.x WAV Milestone

The initial format scope is WAV PCM16. Other formats remain behind explicit
unsupported-format diagnostics until the format-support milestone.

## Feature 1.1: WAV codec trait boundary

Status: implemented.

Implement in `auralis-codec`:

- `AudioReader`
- `AudioWriter`
- `CodecKind`
- unsupported-format errors
- codec capability description

Acceptance tests:

- placeholder formats return unsupported errors;
- WAV is represented as supported only when `auralis-wav` is enabled;
- public documentation describes initial WAV-only scope.

## Feature 1.2: PCM16 WAV decode

Status: implemented.

Implement `auralis-wav` PCM16 WAV reading into planar `f32`.

Acceptance tests:

- decode mono PCM16;
- decode stereo PCM16;
- verify scaling from `i16` to `f32`;
- preserve sample rate and channel count;
- reject unsupported bit depths with typed errors;
- reject malformed WAV gracefully;
- compare decoded samples against Python reference for small fixtures;
- compare at least one mono and one stereo fixture with `sox_ng` converted to
  raw float.

## Feature 1.3: PCM16 WAV encode

Status: implemented.

Implement planar `f32` to PCM16 WAV writing.

Acceptance tests:

- encode mono PCM16;
- encode stereo PCM16;
- clipping behavior documented and tested;
- round-trip generated signals through Auralis read/write;
- compare sample output with SoX-ng for controlled input where applicable.

## Feature 1.4: CLI `inspect`

Status: implemented.

Implement:

```bash
auralis inspect input.wav
```

Output should include:

- format;
- sample rate;
- channels;
- sample format;
- duration frames;
- duration seconds.

Acceptance tests:

- CLI returns expected fields for fixtures;
- invalid path returns non-zero status and clear error;
- unsupported format returns non-zero status and clear error.

## Feature 1.5: CLI copy pipeline

Status: implemented.

Implement:

```bash
auralis run input.wav output.wav
```

The command must decode through internal planar `f32` and re-encode, not simply
copy bytes.

Acceptance tests:

- mono copy sample equivalence within PCM16 quantization tolerance;
- stereo copy sample equivalence within PCM16 quantization tolerance;
- metadata fields are sane;
- unsupported files fail clearly.
