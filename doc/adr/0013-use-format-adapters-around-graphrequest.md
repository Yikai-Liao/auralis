# Use format adapters around GraphRequest

Auralis graph input and machine output formats are thin adapters around the same structured models. TOML and JSON are the planned graph input formats; JSON is the stable machine-readable contract for diagnostics and plans. YAML is not planned now and may be considered later only if it can be added as a small serde adapter without changing graph semantics.

All input formats decode into `GraphRequest`. All output formats encode canonical diagnostics, canonical graph data, or `ExecutionPlan`. No format may have its own validation path, graph semantics, parameter parsing, or execution behavior.

Output selection uses `--format text|json|toml`. `text` is human-readable and may evolve for clarity. `json` is the stable machine-readable contract. `toml` is an adapter over the same canonical data, not a separate contract.

Graph input format is inferred from file extension: `.toml` uses TOML and
`.json` uses JSON. Unknown extensions are errors. There is no separate
`--input-format` flag; `--format` always means output format.

**Considered Options**

- Give each file format its own graph parser and behavior: rejected because it would duplicate validation and create format-specific semantics.
- Keep formats as adapters over `GraphRequest` and canonical outputs: accepted because TOML, JSON, and any future serde-compatible format can share the same graph engine and diagnostics.
