"""Shared JSON failure report schema for Python golden runners."""

from __future__ import annotations

import json
import math
import subprocess
import tomllib
from pathlib import Path
from typing import Any

import numpy as np

from auralis_testkit.metrics import max_abs_error, peak, rms_error, snr_db

GOLDEN_FAILURE_REPORT_SCHEMA = "auralis.golden.failure.v1"


def audio_metadata(sample_rate: int, samples: np.ndarray) -> dict[str, int]:
    """Return decoded stream metadata for a sample array."""

    frame_count, channel_count = sample_shape(samples)
    return {
        "sample_rate": int(sample_rate),
        "channel_count": channel_count,
        "frame_count": frame_count,
    }


def comparison_metadata(
    auralis_rate: int,
    auralis_samples: np.ndarray,
    sox_rate: int,
    sox_samples: np.ndarray,
) -> dict[str, dict[str, int]]:
    """Return output metadata for both sides of a golden comparison."""

    return {
        "auralis": audio_metadata(auralis_rate, auralis_samples),
        "sox_ng": audio_metadata(sox_rate, sox_samples),
    }


def sample_shape(samples: np.ndarray) -> tuple[int, int]:
    """Return frame count and channel count for mono or interleaved samples."""

    if samples.ndim == 1:
        return int(samples.shape[0]), 1
    return int(samples.shape[0]), int(samples.shape[1])


def golden_metrics(reference: np.ndarray, actual: np.ndarray) -> dict[str, float]:
    """Return the standard L2 decoded-sample golden metrics."""

    return {
        "max_abs": max_abs_error(reference, actual),
        "rms": rms_error(reference, actual),
        "snr_db": snr_db(reference, actual),
        "peak_delta": abs(peak(reference) - peak(actual)),
    }


def golden_metric_failures(
    metrics: dict[str, float],
    thresholds: dict[str, float],
    reference: np.ndarray,
    actual: np.ndarray,
) -> list[dict[str, Any]]:
    """Return schema-shaped metric failures for a golden comparison."""

    failures: list[dict[str, Any]] = []
    max_abs_threshold = float(thresholds["max_abs"])
    if metrics["max_abs"] > max_abs_threshold:
        failures.append(
            _metric_failure(
                "max_abs",
                max_abs_threshold,
                metrics["max_abs"],
                "<=",
                first_offending_index(reference, actual, max_abs_threshold),
            )
        )
    if metrics["rms"] > float(thresholds["rms"]):
        failures.append(_metric_failure("rms", thresholds["rms"], metrics["rms"], "<=", None))
    if metrics["snr_db"] < float(thresholds["snr_db"]):
        failures.append(
            _metric_failure("snr_db", thresholds["snr_db"], metrics["snr_db"], ">=", None)
        )
    if metrics["peak_delta"] > max_abs_threshold:
        failures.append(
            _metric_failure("peak_delta", max_abs_threshold, metrics["peak_delta"], "<=", None)
        )
    return failures


def build_golden_failure_report(
    *,
    case_id: str,
    case: dict[str, Any],
    backend: str,
    repo_root: Path,
    auralis_command: list[str],
    sox_ng_command: list[str],
    outputs: dict[str, dict[str, int]],
    metrics: dict[str, float],
    failures: list[dict[str, Any]],
) -> dict[str, Any]:
    """Build a schema-shaped golden failure artifact."""

    inputs = [str(value) for value in case.get("inputs", [case["input"]])]
    corpus_ids = [str(value) for value in case.get("corpus_ids", [case["corpus_id"]])]
    return {
        "schema": GOLDEN_FAILURE_REPORT_SCHEMA,
        "case_id": case_id,
        "backend": backend,
        "auralis_version": auralis_version(repo_root),
        "sox_ng_version": sox_ng_version(sox_ng_command[0]),
        "inputs": inputs,
        "corpus_ids": corpus_ids,
        "auralis_command": auralis_command,
        "sox_ng_command": sox_ng_command,
        "thresholds": {
            "max_abs": float(case["max_abs"]),
            "rms": float(case["rms"]),
            "snr_db": float(case["snr_db"]),
        },
        "outputs": outputs,
        "metrics": {name: json_number(value) for name, value in metrics.items()},
        "failures": failures,
    }


def write_golden_failure_report(path: Path, report: dict[str, Any]) -> None:
    """Write a golden failure artifact as deterministic JSON."""

    path.write_text(json.dumps(report, indent=2, sort_keys=True), encoding="utf-8")


def first_offending_index(
    reference: np.ndarray,
    actual: np.ndarray,
    max_abs_threshold: float,
) -> int | None:
    """Return the first flattened sample index above a max-abs threshold."""

    reference_flat = np.ravel(reference)
    actual_flat = np.ravel(actual)
    limit = max(reference_flat.size, actual_flat.size)
    for index in range(limit):
        reference_value = reference_flat[index] if index < reference_flat.size else 0.0
        actual_value = actual_flat[index] if index < actual_flat.size else 0.0
        if abs(float(reference_value) - float(actual_value)) > max_abs_threshold:
            return index
    return None


def json_number(value: float) -> float | str:
    """Return a JSON-safe representation for a metric value."""

    if math.isfinite(value):
        return value
    return "inf" if value > 0 else "-inf"


def auralis_version(repo_root: Path) -> str:
    """Return the workspace package version recorded in Cargo.toml."""

    cargo_toml = tomllib.loads((repo_root / "Cargo.toml").read_text(encoding="utf-8"))
    return str(cargo_toml["workspace"]["package"]["version"])


def sox_ng_version(executable: str) -> str:
    """Return a best-effort SoX-ng version string."""

    try:
        result = subprocess.run(
            [executable, "--version"],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return "unknown"
    version_text = (result.stdout or result.stderr).strip()
    if not version_text:
        return "unknown"
    return version_text.splitlines()[0]


def _metric_failure(
    metric: str,
    expected: float,
    actual: float,
    comparison: str,
    first_index: int | None,
) -> dict[str, Any]:
    return {
        "metric": metric,
        "expected": json_number(float(expected)),
        "actual": json_number(float(actual)),
        "comparison": comparison,
        "first_offending_index": first_index,
    }
