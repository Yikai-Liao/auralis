"""Checks for the checked-in L0-L7 layered coverage matrix."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path
from typing import Any

import tomllib


REPO_ROOT = Path(__file__).resolve().parents[3]


def test_layered_coverage_matrix_matches_implemented_surface() -> None:
    result = subprocess.run(
        [sys.executable, str(REPO_ROOT / "tools" / "check_layered_coverage.py")],
        cwd=REPO_ROOT,
        check=False,
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0, result.stderr


def test_layered_coverage_gate_writes_report_artifact(tmp_path: Path) -> None:
    report_path = tmp_path / "layered-coverage.json"

    result = subprocess.run(
        [
            sys.executable,
            str(REPO_ROOT / "tools" / "check_layered_coverage.py"),
            "--report",
            str(report_path),
        ],
        cwd=REPO_ROOT,
        check=False,
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0, result.stderr
    assert "wrote report" in result.stdout

    report = load_json_report(report_path)
    matrix = load_matrix()
    subjects = matrix["subject"]
    assert report["schema"] == "auralis.layered_coverage.report.v1"
    assert report["matrix"] == "doc/development/05-layered-coverage.toml"
    assert report["subject_count"] == len(subjects)
    assert sorted(report["kind_counts"]) == ["effect", "pipeline", "primitive"]
    assert set(report["layer_totals"]) == {f"l{layer}" for layer in range(8)}
    assert report["subjects"] == sorted(report["subjects"], key=lambda row: row["id"])


def load_json_report(path: Path) -> dict[str, Any]:
    import json

    with path.open(encoding="utf-8") as report:
        data = json.load(report)
    assert isinstance(data, dict)
    return data


def load_matrix() -> dict[str, Any]:
    with (REPO_ROOT / "doc" / "development" / "05-layered-coverage.toml").open(
        "rb"
    ) as matrix:
        data = tomllib.load(matrix)
    assert isinstance(data, dict)
    return data
