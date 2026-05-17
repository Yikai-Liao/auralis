# Keep CLI as an optional adapter

Auralis library crates must not depend on CLI parsing. `auralis-cli` and `--fx` text parsing are optional adapter surfaces, while graph-op registration, TOML graph specs, Rust APIs, validation diagnostics, execution planning, and plan JSON remain available without the CLI feature or binary.

Global configuration may hide or expose CLI affordances, but it must not remove graph op registration. This keeps libauralis and future bindings able to use the same graph engine without compiling or depending on command-line parsing code.

**Considered Options**

- Let graph or op crates parse CLI effect strings: rejected because it makes graph unusable as a clean library boundary.
- Make CLI metadata control graph registration: rejected because CLI availability and graph capability are different concerns.
- Keep CLI as an optional adapter: accepted because it preserves a thin command-line shell over the shared graph engine.
