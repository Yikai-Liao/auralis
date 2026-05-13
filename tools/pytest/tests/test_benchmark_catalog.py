from __future__ import annotations

import json
from pathlib import Path

from auralis_testkit.benchmarks import (
    BenchmarkCase,
    BackendMode,
    benchmark_case_catalog,
    build_report_summary,
    failed_required_run_keys,
    load_resume_cases,
    parse_args,
    render_markdown_report,
    supported_effect_names,
)

REPO_ROOT = Path(__file__).resolve().parents[3]


def test_benchmark_catalog_covers_every_supported_effect() -> None:
    supported = supported_effect_names(REPO_ROOT)
    cases = benchmark_case_catalog(REPO_ROOT)

    assert [case.effect_name for case in cases] == supported
    assert len(cases) == 63


def test_benchmark_catalog_marks_backend_capability_and_profile_needs() -> None:
    cases = {case.effect_name: case for case in benchmark_case_catalog(REPO_ROOT)}

    assert cases["gain"].backend_mode is BackendMode.SCALAR_AND_SIMD
    assert cases["chorus"].backend_mode is BackendMode.SCALAR_ONLY
    assert cases["reverse"].tokens == ("reverse",)
    assert cases["noisered"].needs_profile is True
    assert cases["noisered"].resolve_tokens(REPO_ROOT / "tmp/profile.prof") == (
        "noisered",
        str(REPO_ROOT / "tmp/profile.prof"),
        "0.25",
    )


def test_render_markdown_report_smoke() -> None:
    report = {
        "generated_at_utc": "2026-05-13T00:00:00+00:00",
        "auralis_version": "0.0.0",
        "sox_ng_version": "sox_ng 14.4.3",
        "config": {
            "duration_seconds": 90,
            "sample_rate": 48_000,
            "input_channels": 2,
            "input_frames": 4_320_000,
            "iterations": 5,
            "warmups": 1,
        },
        "cases": [
            {
                "case_id": "gain",
                "effect_name": "gain",
                "backend_mode": "scalar_and_simd",
                "status": "ok",
                "result_source": "reused",
                "runs": {
                    "sox_ng": {"status": "ok", "summary": {"median_ms": 11.0}},
                    "auralis_scalar": {"status": "ok", "summary": {"median_ms": 8.0}},
                    "auralis_simd": {"status": "ok", "summary": {"median_ms": 5.0}},
                },
                "comparisons": {
                    "scalar_vs_sox_ng": {"median_ratio": 0.727},
                    "simd_vs_sox_ng": {"median_ratio": 0.455},
                    "simd_vs_scalar": {"median_ratio": 0.625},
                },
            }
        ],
    }
    report["summary"] = build_report_summary(report["cases"])

    markdown = render_markdown_report(report)

    assert "SoX-ng Benchmark Report" in markdown
    assert "- Input: 90s @ 48000 Hz, 2 channels, 4320000 frames" in markdown
    assert "- Cases: 1/1 completed successfully" in markdown
    assert "- Reused completed cases: 1" in markdown
    assert "- Scalar vs SoX-ng: 1 faster, 0 slower, 0 equal" in markdown
    assert "- Best SIMD speedup vs SoX-ng: gain (2.198x, ratio 0.455)" in markdown
    assert "| gain | scalar_and_simd | 11.0 | 8.0 | 5.0 | 0.727 | 0.455 | 0.625 |" in markdown


def test_parse_args_rejects_startup_dominated_duration() -> None:
    try:
        parse_args(["--duration-seconds", "3"])
    except SystemExit as error:
        assert error.code == 2
    else:
        raise AssertionError("expected tiny benchmark input duration to be rejected")


def test_build_report_summary_counts_faster_slower_and_na() -> None:
    cases = [
        {
            "case_id": "gain",
            "effect_name": "gain",
            "backend_mode": "scalar_and_simd",
            "status": "ok",
            "result_source": "reused",
            "runs": {
                "auralis_simd": {"status": "ok", "summary": {"median_ms": 5.0}},
            },
            "comparisons": {
                "scalar_vs_sox_ng": {"median_ratio": 0.727, "interpretation": "faster"},
                "simd_vs_sox_ng": {"median_ratio": 0.455, "interpretation": "faster"},
            },
        },
        {
            "case_id": "chorus",
            "effect_name": "chorus",
            "backend_mode": "scalar_only",
            "status": "ok",
            "runs": {
                "auralis_simd": {
                    "status": "not_applicable",
                    "reason": "scalar only",
                },
            },
            "comparisons": {
                "scalar_vs_sox_ng": {"median_ratio": 1.1, "interpretation": "slower"},
                "simd_vs_sox_ng": None,
            },
        },
        {
            "case_id": "oops",
            "effect_name": "oops",
            "backend_mode": "scalar_only",
            "status": "preparation_failed",
        },
    ]

    summary = build_report_summary(cases)

    assert summary["total_cases"] == 3
    assert summary["ok_cases"] == 2
    assert summary["failed_cases"] == 1
    assert summary["preparation_failed_cases"] == 1
    assert summary["reused_cases"] == 1
    assert summary["scalar_faster_than_sox_ng"] == 1
    assert summary["scalar_slower_than_sox_ng"] == 1
    assert summary["scalar_equal_to_sox_ng"] == 0
    assert summary["simd_faster_than_sox_ng"] == 1
    assert summary["simd_slower_than_sox_ng"] == 0
    assert summary["simd_equal_to_sox_ng"] == 0
    assert summary["simd_not_applicable_cases"] == 1
    assert summary["fastest_scalar_vs_sox_ng"] == {
        "effect_name": "gain",
        "median_ratio": 0.727,
        "speedup": 1.376,
    }
    assert summary["fastest_simd_vs_sox_ng"] == {
        "effect_name": "gain",
        "median_ratio": 0.455,
        "speedup": 2.198,
    }


def test_failed_required_run_keys_match_backend_requirements() -> None:
    scalar_case = BenchmarkCase(
        effect_name="chorus",
        tokens=("chorus", "0.5", "0.7", "55", "0.4", "0.25", "2", "-t"),
        backend_mode=BackendMode.SCALAR_ONLY,
        token_source="test",
    )
    simd_case = BenchmarkCase(
        effect_name="gain",
        tokens=("gain", "-n"),
        backend_mode=BackendMode.SCALAR_AND_SIMD,
        token_source="test",
    )
    scalar_only_runs = {
        "sox_ng": {"status": "ok"},
        "auralis_scalar": {"status": "ok"},
        "auralis_simd": {"status": "not_applicable"},
    }

    assert failed_required_run_keys(scalar_case, scalar_only_runs) == []
    assert failed_required_run_keys(simd_case, scalar_only_runs) == ["auralis_simd"]
    assert failed_required_run_keys(scalar_case, {"sox_ng": {"status": "failed"}}) == [
        "sox_ng",
        "auralis_scalar",
    ]


def test_load_resume_cases_reuses_matching_successes_only(tmp_path: Path) -> None:
    output_dir = tmp_path / "bench"
    output_dir.mkdir()
    input_path = output_dir / "benchmark_input.wav"
    report_path = output_dir / "report.json"
    report = {
        "schema": "auralis.sox_ng.benchmark.v1",
        "auralis_version": "0.7.0",
        "sox_ng_version": "sox_ng 14.4.3",
        "config": {
            "repo_root": str(REPO_ROOT),
            "output_dir": str(output_dir),
            "input_path": str(input_path),
            "sample_rate": 48_000,
            "duration_seconds": 90,
            "iterations": 5,
            "warmups": 1,
        },
        "cases": [
            {"case_id": "gain", "effect_name": "gain", "status": "ok"},
            {"case_id": "chorus", "effect_name": "chorus", "status": "failed"},
            {"case_id": "trim", "effect_name": "trim", "status": "ok"},
        ],
    }
    report_path.write_text(json.dumps(report), encoding="utf-8")

    resumed = load_resume_cases(
        report_path,
        repo_root=REPO_ROOT,
        output_dir=output_dir,
        input_path=input_path,
        duration_seconds=90,
        sample_rate=48_000,
        iterations=5,
        warmups=1,
        auralis_version_text="0.7.0",
        sox_version_text="sox_ng 14.4.3",
        selected_effects={"gain", "chorus"},
    )

    assert resumed == {
        "gain": {
            "case_id": "gain",
            "effect_name": "gain",
            "status": "ok",
            "result_source": "reused",
        }
    }


def test_load_resume_cases_rejects_mismatched_config(tmp_path: Path) -> None:
    output_dir = tmp_path / "bench"
    output_dir.mkdir()
    input_path = output_dir / "benchmark_input.wav"
    report_path = output_dir / "report.json"
    report = {
        "schema": "auralis.sox_ng.benchmark.v1",
        "auralis_version": "0.7.0",
        "sox_ng_version": "sox_ng 14.4.3",
        "config": {
            "repo_root": str(REPO_ROOT),
            "output_dir": str(output_dir),
            "input_path": str(input_path),
            "sample_rate": 44_100,
            "duration_seconds": 90,
            "iterations": 5,
            "warmups": 1,
        },
        "cases": [],
    }
    report_path.write_text(json.dumps(report), encoding="utf-8")

    try:
        load_resume_cases(
            report_path,
            repo_root=REPO_ROOT,
            output_dir=output_dir,
            input_path=input_path,
            duration_seconds=90,
            sample_rate=48_000,
            iterations=5,
            warmups=1,
            auralis_version_text="0.7.0",
            sox_version_text="sox_ng 14.4.3",
            selected_effects={"gain"},
        )
    except ValueError as error:
        assert "sample_rate" in str(error)
    else:
        raise AssertionError("expected mismatched resume config to be rejected")
