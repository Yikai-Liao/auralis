## Milestone 6.9: specialized and integration effects

### 6.9 triage rule: specialized effects are not normal implementation items

Before any gnhf iteration starts implementation work in this milestone, classify
the target effect as one of:

- `implemented`: MIT-compatible pure Rust semantics are available, the effect is
  in the registry, diagnostics are stable, and tests cover the accepted behavior.
- `partial`: a MIT-compatible pure Rust subset exists, unsupported SoX-ng
  behavior is documented, diagnostics are stable, and tests lock the supported
  subset.
- `blocked`: SoX-ng-compatible behavior currently depends on GPL code, native
  wrappers, an external host, or unavailable specs.
- `not planned`: the only known path conflicts with current Auralis policy, and
  the effect should not keep re-entering implementation planning.

gnhf must not treat suspicious native-backed, GPL-derived, external-host, or
license-sensitive effects as ordinary implementation leaf features. The first
leaf must be feasibility/classification. Implementation can follow only after
that classification records a MIT-compatible pure Rust path, no native wrapper
dependency, stable user-facing diagnostics, and explicit L0-L7 test expectations.

### Feature 6.9.1: `dolbyb` feasibility and spec

Record whether a safe, testable implementation path exists. If blocked, the CLI
diagnostic must be stable and actionable.

Status: blocked.

The current SoX-ng-compatible implementation path is not safe for Auralis:
SoX-ng wires `dolbyb` through `libdolbyb`, whose C library README states that
the C version is GPLv2, while Auralis is MIT-licensed and the current codec/effect
policy excludes native wrapper dependencies. Auralis also does not vendor or
track a compatible pure-Rust/public-domain Dolby B circuit specification that
could be implemented independently without deriving from the GPLv2 C source.

The `dolbyb` command therefore remains out of the implemented registry. Parser
and CLI diagnostics are intentionally stable and actionable: users should run
`sox_ng ... dolbyb ...` for Dolby B processing, or provide a compatible
pure-Rust/public-domain specification before this effect can be reconsidered.
Feature 6.9.2 must stay blocked until that condition changes.

### Feature 6.9.2: `dolbyb` implementation

Implement only if Feature 6.9.1 records a safe implementation path.

Status: blocked by Feature 6.9.1.

### Feature 6.9.3: `dop`

Classify DoP support before implementing anything.

Status: not planned for the Auralis effect registry.

SoX-ng documents `dop` as DSD over PCM: it packs 1-bit DSD data into 24-bit
samples for transport over non-DSD-aware links. Its implementation rejects
anything except 1-bit input, requires the input rate to be exactly 16 times the
output rate, forces 24-bit output precision, and alternates DoP marker bytes.
That makes `dop` format/transport handling rather than normal PCM audio DSP.

Auralis currently accepts PCM16 WAV input into planar `f32` audio buffers and
writes PCM16 WAV output. It does not have a 1-bit DSD input model, a 24-bit
transport-preserving output path, or a DSD/DoP format boundary where this
packing can be represented without pretending it is an ordinary audio effect.
The registry therefore keeps `dop` out of the implemented effect set and returns
a stable not-planned diagnostic.

SoX-ng compatibility is not meaningful in the current golden effect harness,
because the harness is PCM16 WAV based and cannot express DoP's required 1-bit
DSD input plus 24-bit transport output. L2 golden coverage, scalar DSP, chunk
invariance, and SIMD are N/A until a future DSD/DoP format feature creates the
right bitstream boundary. Reconsideration belongs in format/transport planning,
not in the 6.x PCM effect registry.

Parser, registry visibility, and CLI diagnostics are locked by Rust tests:
`dop` is a known SoX-ng effect, resolves to
`UnsupportedSoxNgEffect { name: "dop" }`, and tells users to run
`sox_ng ... dop ...` for DoP transport or wait for future DSD/DoP format
support.

### Feature 6.9.4: `earwax`

Classify `earwax` as a pure-Rust effect candidate before implementation.

Status: implemented.

The classification pass found a safe implementation path: SoX-ng implements
`earwax` as a no-argument 64-tap interleaved stereo FIR for CD audio, and the
effect source carries a file-local permissive notice allowing redistribution and
use for any purpose. It does not require a native wrapper, GPL-derived external
library, plugin host, or format-specific transport boundary.

Auralis now implements the same restricted surface:

- `earwax` is registered as a typed no-argument effect command.
- Valid input must be stereo 44.1 kHz audio; other channel counts or sample
  rates return a stable `earwax requires stereo audio sampled at 44100 Hz`
  diagnostic.
- The scalar reference path processes samples in SoX-ng's interleaved left/right
  order, preserves the input frame count, and keeps explicit `EarwaxState` for
  chunk-preserving tests.
- L2 golden coverage compares Auralis and SoX-ng on a deterministic stereo
  44.1 kHz corpus fixture.
- SIMD is N/A for this leaf because the accepted behavior is a stateful
  interleaved FIR reference path; vectorized FIR backend planning remains a
  future shared primitive concern.

### Feature 6.9.5: `ladspa` host or stable block

Classify LADSPA before adding any host integration.

Status: blocked.

The current SoX-ng-compatible path is an external native plugin host, not a
pure Rust DSP effect. SoX-ng enables `ladspa` only when LADSPA headers and the
libltdl dynamic-loader path are available, accepts `-l` latency compensation and
`-r` mono-plugin replication, searches `LADSPA_PATH`, opens a plugin module,
resolves `ladspa_descriptor`, selects a plugin label or index, maps control
ports, and runs plugin-provided native code. The behavior, channel count,
latency, control defaults, and license therefore depend on the user's installed
plugins.

Auralis currently has no external plugin-host boundary and must not load native
LADSPA modules, link a host library, or expose plugin ABI details through
`auralis-core`. `ladspa` remains outside the implemented registry until a future
policy deliberately adds an isolated external-host layer.

Parser, registry visibility, and CLI diagnostics are locked by Rust tests:
`ladspa` is a known SoX-ng effect, resolves to
`UnsupportedSoxNgEffect { name: "ladspa" }`, and tells users to run
`sox_ng ... ladspa ...` for LADSPA plugins or wait for a future external-host
boundary. L0 deterministic fixtures, L2 SoX-ng golden comparisons, scalar DSP,
chunk invariance, and SIMD are N/A because Auralis intentionally does not host
native LADSPA plugins in the current pure-Rust effect registry.

### Feature 6.9.6: `sdm` feasibility and spec

Record whether a safe, testable implementation path exists. If blocked, the CLI
diagnostic must be stable and actionable.

Status: not planned for the Auralis effect registry.

SoX-ng's `sdm` is a DSD-oriented sigma-delta modulator. It exposes
`-f {clans|sdm}-[45678]`, trellis order, trellis path count, and output latency
options, selects sample-rate-specific filter tables, and forces the output
precision to 1 bit. SoX-ng also routes `dither -p 1` through the same SDM core.
That behavior is not representable as an ordinary PCM16 WAV effect in Auralis'
current planar `f32` processing model and PCM16 output boundary.

The only currently available SoX-ng-compatible implementation details are the
LGPL `sdm.c` / `sdm.h` source, its static filter coefficient tables, and its
trellis search behavior. Auralis should not port those implementation details
into the MIT-licensed effect registry, and a clean-room pure-Rust implementation
would still need a future DSD/1-bit format boundary before SoX-ng golden
coverage could be meaningful.

Parser, registry visibility, and CLI diagnostics are locked by Rust tests:
`sdm` is a known SoX-ng effect, resolves to
`UnsupportedSoxNgEffect { name: "sdm" }`, and tells users to run
`sox_ng ... sdm ...` for SDM processing or wait for future DSD/1-bit format
support. L0 deterministic fixtures, L2 SoX-ng golden comparisons, scalar DSP,
chunk invariance, and SIMD are N/A in the current effect registry because the
accepted SoX-ng behavior emits 1-bit output outside the PCM16 golden harness.

### Feature 6.9.7: `sdm` implementation

Implement only if Feature 6.9.6 records a safe implementation path.

Status: not planned by Feature 6.9.6.

Implementation is not part of the current PCM16 effect registry. Reconsider SDM
only if a future roadmap adds a DSD/1-bit format boundary and a clean-room,
MIT-compatible pure-Rust specification for the modulator, filter tables, and
trellis behavior.
