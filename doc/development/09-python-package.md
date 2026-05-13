# 9.x Python and Package Milestone

Do not add PyO3 bindings until effect pipeline behavior is stable.

### Feature 9.1: PyO3 package boundary

Status: blocked.

Do not add the `auralis-py` crate, PyO3 bindings, maturin configuration,
NumPy buffer bridge, wheel metadata, or Python package release workflow yet.
This milestone is intentionally blocked until the preconditions below are
re-audited and all pass.

Blocker notes:

- the Rust library API is still pre-alpha and the user-facing effect/format
  surface is still changing;
- effect pipeline behavior is not broad enough to freeze a Python binding
  contract;
- the error and buffer models are designed with future Python exposure in mind
  but are not yet stable public package commitments;
- Python remains an auxiliary test harness, not a shipped binding surface.

Acceptance notes for this blocker:

- no `pyo3`, `maturin`, `numpy`, or `auralis-py` dependency or crate is added;
- Python package work remains listed as future work instead of an immediate
  implementation target;
- README, status, and development docs record that the current autonomous
  feature loop should stop here rather than scaffold unstable bindings.

## Future work

- Python package via PyO3.
- NumPy-compatible buffer interface.
- stable public crate release.

## Preconditions

Only add PyO3 bindings when all are true:

1. Rust library API is stable enough to expose;
2. WAV pipeline is tested;
3. basic effects are tested;
4. error model is stable;
5. buffer model is stable;
6. Python package behavior can be documented clearly;
7. README L0-L7 testing infrastructure is implemented well enough to cover the
   binding boundary.

Before then, Python remains a test harness for corpus generation, golden
comparison, metrics, and failure artifacts.
