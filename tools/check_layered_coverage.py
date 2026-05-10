#!/usr/bin/env python3
"""Validate the checked-in L0-L7 layered coverage matrix."""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from collections import Counter
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[1]
COVERAGE_MATRIX = REPO_ROOT / "doc" / "development" / "05-layered-coverage.toml"
EFFECT_REGISTRY = REPO_ROOT / "crates" / "auralis-effects" / "src" / "registry.rs"
COMBINE_SOURCE = REPO_ROOT / "crates" / "auralis" / "src" / "combine.rs"
EFFECT_GOLDEN_MANIFEST = REPO_ROOT / "tests" / "golden" / "effects.toml"

LAYERS = ("l0", "l1", "l2", "l3", "l4", "l5", "l6", "l7")
STATUSES = ("covered", "not_applicable")


def main() -> int:
    args = parse_args()
    errors = validate()
    if errors:
        for error in errors:
            print(f"layered coverage: {error}", file=sys.stderr)
        return 1
    if args.report is not None:
        report = build_report()
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        print(f"layered coverage: wrote report ({args.report})")
    print(f"layered coverage: ok ({COVERAGE_MATRIX.relative_to(REPO_ROOT)})")
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate the checked-in L0-L7 layered coverage matrix.",
    )
    parser.add_argument(
        "--report",
        type=Path,
        help="write a deterministic JSON coverage-gate artifact",
    )
    return parser.parse_args()


def validate() -> list[str]:
    errors: list[str] = []
    matrix = load_toml(COVERAGE_MATRIX, errors)
    if not matrix:
        return errors

    if matrix.get("schema_version") != 1:
        errors.append("schema_version must be 1")

    subjects = matrix.get("subject")
    if not isinstance(subjects, list) or not subjects:
        errors.append("matrix must contain at least one [[subject]] row")
        return errors

    subject_ids: set[str] = set()
    effect_ids: set[str] = set()
    combine_ids: set[str] = set()

    for index, subject in enumerate(subjects):
        if not isinstance(subject, dict):
            errors.append(f"subject #{index} must be a table")
            continue
        subject_id = require_string(subject, "id", f"subject #{index}", errors)
        kind = require_string(subject, "kind", subject_id, errors)
        require_string(subject, "name", subject_id, errors)

        if subject_id in subject_ids:
            errors.append(f"{subject_id}: duplicate subject id")
        subject_ids.add(subject_id)

        if kind == "effect":
            effect_ids.add(subject_id)
        elif kind == "pipeline":
            if subject_id.startswith("pipeline.combine."):
                combine_ids.add(subject_id)
        elif kind == "primitive":
            pass
        else:
            errors.append(f"{subject_id}: kind must be effect, pipeline, or primitive")

        validate_layers(subject, subject_id, errors)

    expected_effect_ids = {f"effect.{name}" for name in implemented_effect_names()}
    if effect_ids != expected_effect_ids:
        errors.append(
            "effect coverage rows must match supported registry effects: "
            f"expected {sorted(expected_effect_ids)}, found {sorted(effect_ids)}"
        )

    expected_combine_ids = {
        f"pipeline.combine.{name.replace('-', '_')}" for name in implemented_combine_names()
    }
    if combine_ids != expected_combine_ids:
        errors.append(
            "combine coverage rows must match implemented CombineMethod names: "
            f"expected {sorted(expected_combine_ids)}, found {sorted(combine_ids)}"
        )

    golden_effects = effects_in_golden_manifest(errors)
    if golden_effects and golden_effects != implemented_effect_names():
        errors.append(
            "tests/golden/effects.toml must cover every implemented effect: "
            f"expected {sorted(implemented_effect_names())}, found {sorted(golden_effects)}"
        )

    return errors


def build_report() -> dict[str, Any]:
    matrix = load_toml(COVERAGE_MATRIX, [])
    subjects = matrix["subject"]
    kind_counts: Counter[str] = Counter()
    layer_totals: dict[str, Counter[str]] = {layer: Counter() for layer in LAYERS}
    subject_reports: list[dict[str, Any]] = []

    for subject in subjects:
        kind_counts[subject["kind"]] += 1
        layers = subject["layers"]
        layer_statuses: dict[str, str] = {}
        for layer in LAYERS:
            status = layers[layer]["status"]
            layer_statuses[layer] = status
            layer_totals[layer][status] += 1
        subject_reports.append(
            {
                "id": subject["id"],
                "kind": subject["kind"],
                "layers": layer_statuses,
                "name": subject["name"],
            }
        )

    return {
        "schema": "auralis.layered_coverage.report.v1",
        "matrix": str(COVERAGE_MATRIX.relative_to(REPO_ROOT)),
        "schema_version": matrix["schema_version"],
        "subject_count": len(subjects),
        "kind_counts": dict(sorted(kind_counts.items())),
        "layer_totals": {
            layer: {status: layer_totals[layer][status] for status in STATUSES}
            for layer in LAYERS
        },
        "subjects": sorted(subject_reports, key=lambda subject: subject["id"]),
    }


def validate_layers(subject: dict[str, Any], subject_id: str, errors: list[str]) -> None:
    layers = subject.get("layers")
    if not isinstance(layers, dict):
        errors.append(f"{subject_id}: missing [subject.layers] table")
        return

    layer_names = set(layers)
    if layer_names != set(LAYERS):
        errors.append(
            f"{subject_id}: layers must be exactly {', '.join(LAYERS)}; "
            f"found {', '.join(sorted(layer_names))}"
        )

    for layer in LAYERS:
        entry = layers.get(layer)
        if not isinstance(entry, dict):
            errors.append(f"{subject_id}.{layer}: layer entry must be an inline table")
            continue
        status = entry.get("status")
        if status not in STATUSES:
            errors.append(
                f"{subject_id}.{layer}: status must be one of {', '.join(STATUSES)}"
            )
        reason = entry.get("reason")
        if not isinstance(reason, str) or not reason.strip():
            errors.append(f"{subject_id}.{layer}: reason must be a non-empty string")

        tests = entry.get("tests")
        if not isinstance(tests, list) or any(not isinstance(test, str) for test in tests):
            errors.append(f"{subject_id}.{layer}: tests must be a list of paths")
            continue

        if status == "covered" and not tests:
            errors.append(f"{subject_id}.{layer}: covered layers must link tests")
        if status == "not_applicable" and tests:
            errors.append(f"{subject_id}.{layer}: not_applicable layers must not link tests")

        for test_path in tests:
            path = REPO_ROOT / test_path
            if not path.exists():
                errors.append(f"{subject_id}.{layer}: linked test path does not exist: {test_path}")


def implemented_effect_names() -> set[str]:
    source = EFFECT_REGISTRY.read_text(encoding="utf-8")
    return set(
        re.findall(
            r'EffectDescriptor::new\(\s*EffectKind::[A-Za-z]+,\s*"([^"]+)"',
            source,
            flags=re.MULTILINE,
        )
    )


def implemented_combine_names() -> set[str]:
    source = COMBINE_SOURCE.read_text(encoding="utf-8")
    as_name = source.split("pub const fn as_name", maxsplit=1)[1].split(
        "pub fn from_name", maxsplit=1
    )[0]
    return set(re.findall(r'=>\s*"([^"]+)"', as_name))


def effects_in_golden_manifest(errors: list[str]) -> set[str]:
    manifest = load_toml(EFFECT_GOLDEN_MANIFEST, errors)
    cases = manifest.get("id", {}) if manifest else {}
    if not isinstance(cases, dict):
        errors.append("tests/golden/effects.toml must contain [id.*] cases")
        return set()

    effects: set[str] = set()
    for case_id, case in cases.items():
        if not isinstance(case, dict):
            errors.append(f"tests/golden/effects.toml:{case_id}: case must be a table")
            continue
        sox_ng = case.get("sox_ng")
        if not isinstance(sox_ng, list) or not sox_ng or not isinstance(sox_ng[0], str):
            errors.append(f"tests/golden/effects.toml:{case_id}: sox_ng must name an effect")
            continue
        effects.add(sox_ng[0])
    return effects


def require_string(
    table: dict[str, Any], key: str, context: str, errors: list[str]
) -> str:
    value = table.get(key)
    if not isinstance(value, str) or not value.strip():
        errors.append(f"{context}: {key} must be a non-empty string")
        return ""
    return value


def load_toml(path: Path, errors: list[str]) -> dict[str, Any]:
    try:
        with path.open("rb") as file:
            data = tomllib.load(file)
    except OSError as error:
        errors.append(f"{path.relative_to(REPO_ROOT)}: {error}")
        return {}
    except tomllib.TOMLDecodeError as error:
        errors.append(f"{path.relative_to(REPO_ROOT)}: {error}")
        return {}
    if not isinstance(data, dict):
        errors.append(f"{path.relative_to(REPO_ROOT)}: top-level TOML value must be a table")
        return {}
    return data


if __name__ == "__main__":
    raise SystemExit(main())
