---
kind: ci-benchmark-job
job: "external-binary-release"
status: planned
trigger: "manual"
runner: "GitHub Actions Ubuntu"
owner: "release-engineering"
upload_target: "external dependency release repository"
---

# External Binary Release Job

## One-Line Goal

Publish third-party test and benchmark binaries, starting with `sox_ng`, from a
separate release repository for Auralis CI to consume.

## Inputs

- case set: binary build and smoke test for each third-party tool;
- audio shape: tiny deterministic smoke fixture;
- backend: tool-specific;
- third-party binaries: source builds or trusted upstream artifacts;
- feature flags: not applicable.

## Command

```bash
rtk ./scripts/build-sox-ng-release.sh
rtk ./scripts/smoke-sox-ng.sh ./dist/sox_ng
rtk ./scripts/publish-release-artifacts.sh ./dist
```

The scripts live in the external binary repository, not in Auralis, unless a
future decision vendors them here.

## Output Policy

- output directory: external repo `dist/`;
- preserved baseline: not applicable;
- artifacts: binary archive, checksum, build provenance, smoke-test log;
- retention: GitHub release assets.

## Pass/Fail Rule

Build, checksum generation, and smoke test failures block publication.

## Rerun Rule

Rerun manually for tool upgrades, security rebuilds, or platform changes.

## Failure Triage

- missing source dependency: fix external repository build environment;
- smoke mismatch: do not publish;
- checksum mismatch: rebuild and investigate provenance;
- platform gap: publish only supported platform matrix entries.

## Done When

- Release asset includes binary, checksum, version, and provenance.
- Auralis CI can download by pinned version.
- Smoke command proves the binary can run before publication.
