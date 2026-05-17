---
kind: oracle-notes
subject: "dynamics"
status: planned
oracle_type: "sox-ng"
selected_reference: "SoX-ng for compand/mcompand compatibility"
rejected_references:
  - "one oracle for both compatibility compand and modern compressor"
owner: "effects/compand, effects/mcompand, future compressor"
---

# Dynamics Oracle

## One-Line Scope

This oracle separates SoX-compatible companding from future modern compressor,
expander, gate, and limiter effects.

## Why This Oracle

`compand` and `mcompand` are compatibility effects whose value is SoX-like
curve semantics. A future modern dynamics family should instead reference Faust
compressors and use conventional attack, release, knee, ratio, link, and
lookahead controls.

## Why Not The Rejected Reference

Using one oracle for both goals would make the parameter model ambiguous and
would hide whether a test is checking compatibility or modern behavior.

## What The Oracle Proves

For `compand`/`mcompand`, SoX-ng golden fixtures prove compatibility. For future
modern dynamics, Faust-derived fixtures should prove envelope and gain-computer
behavior.

## What It Does Not Prove

It does not prove that SoX compand is a modern compressor target, nor that
future compressor effects preserve SoX curve syntax.

## Fixtures

Use step-level, sine, speech, and music fixtures. Keep SoX-compatible fixtures
separate from modern compressor fixtures.

## Comparison Method

Use SoX-ng decoded-sample or metric comparison for compatibility effects. Use
analytical envelope and gain-computer tests for future modern dynamics.

## Regeneration Rule

Regenerate when compatibility syntax changes or when a modern dynamics effect
selects a concrete reference.

## Done When

Compatibility and modern dynamics tests are not mixed under one effect name.
