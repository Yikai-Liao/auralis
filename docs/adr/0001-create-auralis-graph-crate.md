# Create an auralis-graph crate

Auralis will move graph specs, graph IR, execution planning, and graph execution out of `auralis-cli` into a new `auralis-graph` crate. This keeps the CLI as an adapter over the shared graph execution model, keeps `auralis-core` focused on vocabulary types, and lets the Rust library facade expose graph workflows without depending on CLI-private code.

**Considered Options**

- Keep graph code in `auralis-cli`: rejected because it preserves duplicate CLI planning and blocks Rust API reuse.
- Put graph code directly in `auralis`: rejected because the facade crate would become the engine implementation warehouse.
- Create `auralis-graph`: accepted because the graph engine is a reusable library boundary shared by CLI, Rust API, and future bindings.
