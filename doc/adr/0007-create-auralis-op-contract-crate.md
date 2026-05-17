# Create an auralis-op contract crate

Auralis will introduce a thin `auralis-op` crate for op contracts shared by `auralis-effects`, `auralis-graph`, CLI adapters, and future bindings. It owns descriptors, parameter and port metadata, op names, registries, and prepared-op evaluation contracts, but it does not own graph requests, planning, CLI parsing, DSP kernels, codec I/O, or execution policy.

This avoids two bad dependency directions: `auralis-effects` should not depend on the whole graph planner just to declare `gain`, and `auralis-graph` should not depend on concrete effect modules just to validate and plan op nodes.

**Considered Options**

- Put op contracts in `auralis-graph`: rejected because effect crates would have to depend on graph planning and execution concepts.
- Put op contracts in `auralis-effects`: rejected because graph validation would depend on concrete effect implementations.
- Create `auralis-op`: accepted because it gives graph, effects, CLI, and future bindings one small shared vocabulary without collapsing layer boundaries.
