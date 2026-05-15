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
from auralis_testkit.sox_ng import (
    SoxNgUnavailable,
    run_sox_ng,
    run_sox_ng_with_inputs,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
MANIFEST_PATH = REPO_ROOT / "tests" / "golden" / "chains.toml"
SAMPLE_RATE = 48_000
PCM16_SCALE = np.float64(32_768.0)


def _load_chain_cases() -> tuple[tuple[str, dict[str, Any]], ...]:
    manifest = tomllib.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    return tuple(sorted(manifest["id"].items()))


CHAIN_CASES = _load_chain_cases()


@pytest.mark.golden
@pytest.mark.parametrize(
    ("case_id", "case"),
    CHAIN_CASES,
    ids=[case_id for case_id, _ in CHAIN_CASES],
)
def test_cli_chain_matches_sox_ng_golden_manifest(
    case_id: str,
    case: dict[str, Any],
    tmp_path: Path,
) -> None:
    input_paths = _write_fixtures(case, tmp_path)
    auralis_output = tmp_path / f"{case_id}.auralis.wav"
    auralis_effects_file = tmp_path / f"{case_id}.effects"
    auralis_effects_file_output = tmp_path / f"{case_id}.auralis.effects-file.wav"
    sox_output = tmp_path / f"{case_id}.sox.wav"
    auralis_effects_file.write_text(
        _effects_file_source(case["auralis"]),
        encoding="utf-8",
    )

    auralis_command = [
        "cargo",
        "run",
        "--quiet",
        "--package",
        "auralis-cli",
        "--",
        "render",
        *_auralis_pipeline_args(input_paths, auralis_output, case),
        "--fx",
        " ".join(case["auralis"]),
    ]
    auralis_effects_file_command = [
        "cargo",
        "run",
        "--quiet",
        "--package",
        "auralis-cli",
        "--",
        "render",
        *_auralis_pipeline_args(input_paths, auralis_effects_file_output, case),
        "--effects-file",
        str(auralis_effects_file),
    ]
    auralis_result = subprocess.run(
        auralis_command,
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    auralis_effects_file_result = subprocess.run(
        auralis_effects_file_command,
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    )

    try:
        sox_result = _run_sox_ng_pipeline(input_paths, sox_output, case)
    except SoxNgUnavailable as error:
        pytest.skip(str(error))

    auralis_rate, auralis_samples = _read_pcm16(auralis_output)
    auralis_effects_file_rate, auralis_effects_file_samples = _read_pcm16(
        auralis_effects_file_output
    )
    sox_rate, sox_samples = _read_pcm16(sox_output)
    assert auralis_effects_file_rate == auralis_rate
    np.testing.assert_array_equal(auralis_effects_file_samples, auralis_samples)
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
        report["auralis_effects_file_command"] = auralis_effects_file_result.args
        if "combine" in case:
            report["combine"] = case["combine"]
        write_golden_failure_report(report_path, report)
        pytest.fail(_failure_summary(failures) + f"; report={report_path}")


def _effects_file_source(args: list[str]) -> str:
    return "# generated from the golden manifest's Auralis effect arguments\n" + " ".join(
        args
    ) + "\n"


def _write_fixtures(case: dict[str, Any], tmp_path: Path) -> list[Path]:
    if "inputs" in case:
        return [
            _write_fixture(corpus_id, tmp_path / input_name)
            for input_name, corpus_id in zip(case["inputs"], case["corpus_ids"], strict=True)
        ]
    return [_write_fixture(case["corpus_id"], tmp_path / case["input"])]


def _write_fixture(corpus_id: str, path: Path) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    return pcm16_corpus_fixture(path, corpus_id)


def _auralis_pipeline_args(
    input_paths: list[Path],
    output_path: Path,
    case: dict[str, Any],
) -> list[str]:
    args = [str(input_paths[0]), "-o", str(output_path)]
    if len(input_paths) > 1:
        args.extend(["--combine", case.get("combine", "concatenate")])
        for input_path in input_paths[1:]:
            args.extend(["--input", str(input_path)])
    if "output_channels" in case:
        args.extend(["--channels", str(case["output_channels"])])
    if "output_sample_rate" in case:
        args.extend(["--rate", str(case["output_sample_rate"])])
    return args


def _run_sox_ng_pipeline(
    input_paths: list[Path],
    output_path: Path,
    case: dict[str, Any],
) -> subprocess.CompletedProcess[bytes]:
    kwargs = {
        "output_channels": case.get("output_channels"),
        "output_sample_rate": case.get("output_sample_rate"),
        "disable_auto_dither": not case.get("sox_ng_auto_dither", False),
    }
    if len(input_paths) > 1:
        return run_sox_ng_with_inputs(
            input_paths,
            output_path,
            case["sox_ng"],
            combine=case.get("combine", "concatenate"),
            **kwargs,
        )
    return run_sox_ng(input_paths[0], output_path, case["sox_ng"], **kwargs)


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
