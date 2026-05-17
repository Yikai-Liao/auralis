# Use named op parameters only

Auralis will use named op parameters across CLI `--fx`, `GraphRequest`, TOML graph specs, Rust helpers, and future bindings. Positional effect parameters are not part of the target CLI or graph model; existing SoX-style positional parsing is migration debt, not a compatibility constraint for new design.

Examples:

```bash
auralis render input.wav -o output.wav --fx gain,by=-3dB --fx fade,out=500ms
```

```toml
[[nodes]]
id = "clean"
op = "gain"
input = "voice.audio"
by = "-3dB"
```

**Considered Options**

- Keep positional effect syntax for compatibility: rejected because this repository has no CLI compatibility burden and positional parsing is a major source of parser and registry complexity.
- Use named parameters only: accepted because it gives CLI, graph specs, Rust helpers, diagnostics, schema generation, and future bindings one shared parameter model.
- Use space-separated `--fx gain by=-3dB`: rejected because repeated variable-length CLI arguments create boundary and negative-value ambiguity.

CLI `--fx` uses one comma-separated value per op. The first segment is the op name; remaining segments are `key=value` named parameters. Space-separated effect parameters are not a target syntax.

CLI does not perform final unit parsing. For example, `--fx gain,by=-3dB`
lowers to an op name plus a parameter value carrying `"-3dB"`. Graph
validation/prepare converts that value into the typed `Db` field used by
`GainParams`, using the same parser as TOML, Rust helper, and future binding
frontends.
