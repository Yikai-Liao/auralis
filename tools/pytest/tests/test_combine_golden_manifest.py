from __future__ import annotations

import subprocess
import tomllib
from pathlib import Path
from typing import Any

import numpy as np
import pytest
from scipy.io import wavfile

from auralis_testkit.corpus import pcm16_corpus_fixture
from auralis_testkit.golden_report import (
    build_golden_failure_report,
    comparison_metadata,
    golden_metric_failures,
    golden_metrics,
    write_golden_failure_report,
)
from auralis_testkit.sox_ng import SoxNgUnavailable, run_sox_ng_with_inputs

REPO_ROOT = Path(__file__).resolve().parents[3]
MANIFEST_PATHS = (
    REPO_ROOT / "tests" / "golden" / "concat.toml",
    REPO_ROOT / "tests" / "golden" / "sequence.toml",
    REPO_ROOT / "tests" / "golden" / "mix.toml",
    REPO_ROOT / "tests" / "golden" / "mix_power.toml",
    REPO_ROOT / "tests" / "golden" / "merge.toml",
    REPO_ROOT / "tests" / "golden" / "multiply.toml",
)
SAMPLE_RATE = 48_000
PCM16_SCALE = np.float64(32_768.0)


def _load_combine_cases() -> tuple[tuple[str, dict[str, Any]], ...]:
    cases: list[tuple[str, dict[str, Any]]] = []
    for manifest_path in MANIFEST_PATHS:
        manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
        for case_id, case in manifest["id"].items():
            case = dict(case)
            case.setdefault("combine", "concatenate")
            cases.append((case_id, case))

    return tuple(sorted(cases))


COMBINE_CASES = _load_combine_cases()


@pytest.mark.golden
@pytest.mark.parametrize(
    ("case_id", "case"),
    COMBINE_CASES,
    ids=[case_id for case_id, _ in COMBINE_CASES],
)
def test_cli_combine_matches_sox_ng_golden_manifest(
    case_id: str,
    case: dict[str, Any],
    tmp_path: Path,
) -> None:
    combine = case["combine"]
    input_paths = [
        _write_fixture(corpus_id, tmp_path / input_name)
        for input_name, corpus_id in zip(case["inputs"], case["corpus_ids"], strict=True)
    ]
    auralis_output = tmp_path / f"{case_id}.auralis.wav"
    sox_output = tmp_path / f"{case_id}.sox.wav"
    auralis_command = [
        "cargo",
        "run",
        "--quiet",
        "--package",
        "auralis-cli",
        "--",
        "render",
        str(input_paths[0]),
        "-o",
        str(auralis_output),
        "--combine",
        combine,
        *[
            option
            for input_path in input_paths[1:]
            for option in ("--input", str(input_path))
        ],
        *_auralis_effect_args(case["auralis"]),
    ]
    auralis_result = subprocess.run(
        auralis_command,
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    )

    try:
        sox_result = run_sox_ng_with_inputs(
            input_paths,
            sox_output,
            case["sox_ng"],
            combine=combine,
        )
    except SoxNgUnavailable as error:
        pytest.skip(str(error))

    auralis_rate, auralis_samples = _read_pcm16(auralis_output)
    sox_rate, sox_samples = _read_pcm16(sox_output)
    metadata = comparison_metadata(auralis_rate, auralis_samples, sox_rate, sox_samples)

    assert metadata["auralis"]["sample_rate"] == metadata["sox_ng"]["sample_rate"]
    assert metadata["auralis"]["channel_count"] == metadata["sox_ng"]["channel_count"]
    assert metadata["auralis"]["frame_count"] == metadata["sox_ng"]["frame_count"]

    reference = _normalize_pcm16(sox_samples)
    actual = _normalize_pcm16(auralis_samples)
    metrics = golden_metrics(reference, actual)
    failures = golden_metric_failures(metrics, case, reference, actual)

    if failures:
        report_path = tmp_path / f"{case_id}.failure.json"
        report = build_golden_failure_report(
            case_id=case_id,
            case=case,
            backend="scalar",
            repo_root=REPO_ROOT,
            auralis_command=auralis_result.args,
            sox_ng_command=sox_result.args,
            outputs=metadata,
            metrics=metrics,
            failures=failures,
        )
        report["combine"] = combine
        write_golden_failure_report(report_path, report)
        pytest.fail(_failure_summary(failures) + f"; report={report_path}")


def _write_fixture(corpus_id: str, path: Path) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    return pcm16_corpus_fixture(path, corpus_id)


def _auralis_effect_args(args: list[str]) -> list[str]:
    if not args:
        return []
    return ["--fx", " ".join(args)]


def _read_pcm16(path: Path) -> tuple[int, np.ndarray]:
    sample_rate, samples = wavfile.read(path)
    assert samples.dtype == np.int16
    return sample_rate, samples


def _normalize_pcm16(samples: np.ndarray) -> np.ndarray:
    return samples.astype(np.float64) / PCM16_SCALE


def _failure_summary(failures: list[dict[str, Any]]) -> str:
    return "; ".join(
        f"{failure['metric']} {failure['actual']} {failure['comparison']} {failure['expected']}"
        for failure in failures
    )
