# Use per-effect oracle policy

Auralis will not treat SoX-ng as the default quality oracle for every effect. Each effect must declare an oracle policy: `sox-ng`, `analytical`, `modern-reference`, or `not-planned`. SoX-ng remains useful only where its implementation quality and semantics are still worth matching; effects with weak, obsolete, or semantically wrong SoX-ng implementations should be planned against modern target algorithms instead of preserving SoX compatibility.

Development plans and source comments must list the target reference for each `modern-reference` effect and define a migration path for tests and benchmarks away from SoX-ng. This metadata is documentation, not runtime graph configuration and not an op execution contract. It should not be moved into op runtime code or a separate policy TOML unless a future automation need justifies that added structure.

**Considered Options**

- Keep SoX-ng as the default oracle for all comparable effects: rejected because several SoX-ng implementations are not suitable quality or semantic targets for Auralis.
- Choose oracle policy per effect: accepted because it preserves good SoX-ng references where useful while allowing Auralis to target modern algorithms where SoX-ng is the wrong reference.
