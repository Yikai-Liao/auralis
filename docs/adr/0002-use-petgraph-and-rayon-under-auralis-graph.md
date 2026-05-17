# Use petgraph and rayon under auralis-graph

`auralis-graph` will use `petgraph` for graph storage, traversal, topological ordering, cycle detection, and graph-debug output, and `rayon` for CPU-bound parallel execution under a deterministic Auralis-owned execution plan. Auralis still owns audio-domain node types, validation, cache boundaries, barrier and fusion policy, deterministic floating-point ordering where required, and the public graph contract.

**Considered Options**

- Hand-write graph algorithms and worker scheduling: rejected because mature Rust crates already cover the generic graph and CPU-parallel substrate.
- Adopt a full async or dataflow runtime as the core executor: rejected because those runtimes would impose the wrong domain model for deterministic offline audio rendering.
- Use `petgraph` plus `rayon` behind Auralis-owned wrappers: accepted because it avoids low-level reinvention while preserving Auralis-specific graph semantics.
