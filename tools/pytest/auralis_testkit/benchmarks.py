"""Release benchmark helpers for comparing Auralis with SoX-ng."""

from __future__ import annotations

import argparse
import json
import math
import re
import shlex
import statistics
import subprocess
import tempfile
import time
from dataclasses import dataclass
from datetime import UTC, datetime
from enum import StrEnum
from pathlib import Path
from typing import Any

import numpy as np

from auralis_testkit.corpus import pcm16_fixture
from auralis_testkit.golden_report import auralis_version, json_number, sox_ng_version
from auralis_testkit.sox_ng import find_sox_ng

BENCHMARK_REPORT_SCHEMA = "auralis.sox_ng.benchmark.v1"
RELEASE_BUILD_COMMAND = ("cargo", "build", "--release", "-p", "auralis-cli", "--features", "simd")
DEFAULT_ITERATIONS = 5
DEFAULT_WARMUPS = 1
DEFAULT_DURATION_SECONDS = 90
MIN_DURATION_SECONDS = 30
DEFAULT_SAMPLE_RATE = 48_000
BENCHMARK_INPUT_CHANNELS = 2
EARWAX_REQUIRED_SAMPLE_RATE = 44_100
EARWAX_REQUIRED_CHANNELS = 2
DEFAULT_OUTPUT_DIR = Path("target/benchmarks/sox_ng")
REPORT_FILENAME = "report.json"
PROFILE_PLACEHOLDER = "profile.prof"
MANUAL_EFFECT_TOKENS = {
    "reverse": ("reverse",),
}
FUZZ_CASE_FILE_OVERRIDES = {
    "dcshift": "dcshift_limiter",
    "fade": "fade_curves",
    "pad": "pad_positioned",
    "trim": "trim_multi",
    "vol": "vol_limiter",
}
SIMD_EFFECTS = frozenset({"channels", "dcshift", "fade", "gain", "norm", "vol"})


class BackendMode(StrEnum):
    """Whether an effect has a distinct SIMD-aware implementation path."""

    SCALAR_ONLY = "scalar_only"
    SCALAR_AND_SIMD = "scalar_and_simd"


@dataclass(frozen=True)
class BenchmarkCase:
    """One runnable benchmark case for an implemented effect."""

    effect_name: str
    tokens: tuple[str, ...]
    backend_mode: BackendMode
    token_source: str

    @property
    def case_id(self) -> str:
        """Return the stable benchmark case identifier."""

        return self.effect_name

    @property
    def needs_profile(self) -> bool:
        """Return whether the case references an external noise profile."""

        return PROFILE_PLACEHOLDER in self.tokens

    def resolve_tokens(self, profile_path: Path | None = None) -> tuple[str, ...]:
        """Resolve any placeholder paths in the case tokens."""

        if not self.needs_profile:
            return self.tokens
        if profile_path is None:
            raise ValueError(f"{self.effect_name} requires a profile path")
        profile_text = str(profile_path)
        return tuple(
            profile_text if token == PROFILE_PLACEHOLDER else token for token in self.tokens
        )


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    """Parse benchmark CLI arguments."""

    parser = argparse.ArgumentParser(
        description=(
            "Benchmark all implemented Auralis effects against SoX-ng in release mode, "
            "with separate scalar and SIMD coverage where Auralis has a real SIMD path."
        )
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[3],
        help="repository root containing Cargo.toml",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=DEFAULT_OUTPUT_DIR,
        help="directory where benchmark inputs and reports are written",
    )
    parser.add_argument(
        "--effect",
        action="append",
        default=[],
        help="limit the run to one effect name; repeat to include multiple effects",
    )
    parser.add_argument(
        "--iterations",
        type=positive_int,
        default=DEFAULT_ITERATIONS,
        help="measured iterations per tool/effect",
    )
    parser.add_argument(
        "--warmups",
        type=nonnegative_int,
        default=DEFAULT_WARMUPS,
        help="warmup iterations per tool/effect",
    )
    parser.add_argument(
        "--duration-seconds",
        type=benchmark_duration_seconds,
        default=DEFAULT_DURATION_SECONDS,
        help=f"shared benchmark input duration in seconds; minimum {MIN_DURATION_SECONDS}s",
    )
    parser.add_argument(
        "--sample-rate",
        type=positive_int,
        default=DEFAULT_SAMPLE_RATE,
        help="shared benchmark input sample rate in Hz",
    )
    parser.add_argument(
        "--list-cases",
        action="store_true",
        help="print the benchmark case catalog and exit",
    )
    parser.add_argument(
        "--skip-build",
        action="store_true",
        help="assume target/release/auralis already exists instead of rebuilding it",
    )
    parser.add_argument(
        "--resume",
        action="store_true",
        help="reuse successful cases from an existing report.json with the same benchmark config",
    )
    return parser.parse_args(argv)


def benchmark_case_catalog(repo_root: Path) -> list[BenchmarkCase]:
    """Return the full benchmark catalog in supported-effect order."""

    fuzz_root = repo_root / "fuzz/corpus/effect_command"
    cases: list[BenchmarkCase] = []

    for effect_name in supported_effect_names(repo_root):
        if effect_name in MANUAL_EFFECT_TOKENS:
            tokens = MANUAL_EFFECT_TOKENS[effect_name]
            token_source = "manual"
        else:
            fuzz_name = FUZZ_CASE_FILE_OVERRIDES.get(effect_name, effect_name)
            fuzz_path = fuzz_root / fuzz_name
            if not fuzz_path.is_file():
                raise FileNotFoundError(f"missing benchmark command seed for {effect_name}: {fuzz_path}")
            tokens = read_first_command_line(fuzz_path)
            if tokens[0] != effect_name:
                raise ValueError(
                    f"benchmark command seed {fuzz_path} resolves to {tokens[0]!r}, expected {effect_name!r}"
                )
            token_source = str(fuzz_path.relative_to(repo_root))

        cases.append(
            BenchmarkCase(
                effect_name=effect_name,
                tokens=tokens,
                backend_mode=(
                    BackendMode.SCALAR_AND_SIMD
                    if effect_name in SIMD_EFFECTS
                    else BackendMode.SCALAR_ONLY
                ),
                token_source=token_source,
            )
        )

    return cases


def supported_effect_names(repo_root: Path) -> list[str]:
    """Extract supported canonical effect names from the Rust registry source."""

    registry_path = repo_root / "crates/auralis-effects/src/registry.rs"
    registry_text = registry_path.read_text(encoding="utf-8")
    names = re.findall(
        r'EffectDescriptor::new\(\s*EffectKind::\w+,\s*"([^"]+)"',
        registry_text,
    )
    if not names:
        raise ValueError(f"could not extract supported effects from {registry_path}")
    return names


def read_first_command_line(path: Path) -> tuple[str, ...]:
    """Return the first non-empty command line from a fuzz corpus seed."""

    for line in path.read_text(encoding="utf-8").splitlines():
        command = line.strip()
        if command:
            return tuple(shlex.split(command))
    raise ValueError(f"fuzz seed {path} does not contain a benchmarkable command line")


def run_benchmarks(args: argparse.Namespace) -> dict[str, Any]:
    """Run the benchmark suite and return the JSON report payload."""

    repo_root = args.repo_root.resolve()
    output_dir = (repo_root / args.output_dir).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    cases = benchmark_case_catalog(repo_root)
    selected_effects = set(args.effect)
    if selected_effects:
        cases = [case for case in cases if case.effect_name in selected_effects]
        missing = sorted(selected_effects - {case.effect_name for case in cases})
        if missing:
            raise ValueError(f"unknown effect filters: {', '.join(missing)}")

    sox_executable = find_sox_ng()
    if sox_executable is None:
        raise RuntimeError("sox_ng is not available in PATH or AURALIS_SOX_NG_BIN")

    auralis_bin = ensure_release_auralis(repo_root, skip_build=args.skip_build)
    input_path = output_dir / "benchmark_input.wav"
    write_benchmark_input(
        input_path,
        duration_seconds=args.duration_seconds,
        sample_rate=args.sample_rate,
    )
    auralis_version_text = auralis_version(repo_root)
    sox_version_text = sox_ng_version(sox_executable)
    resumed_cases = (
        load_resume_cases(
            output_dir / REPORT_FILENAME,
            repo_root=repo_root,
            output_dir=output_dir,
            input_path=input_path,
            duration_seconds=args.duration_seconds,
            sample_rate=args.sample_rate,
            iterations=args.iterations,
            warmups=args.warmups,
            auralis_version_text=auralis_version_text,
            sox_version_text=sox_version_text,
            selected_effects={case.effect_name for case in cases},
        )
        if args.resume
        else {}
    )

    case_reports = []
    for case in cases:
        resumed = resumed_cases.get(case.case_id)
        if resumed is not None:
            case_reports.append(resumed)
            continue
        case_reports.append(
            benchmark_case(
                case,
                repo_root=repo_root,
                auralis_bin=auralis_bin,
                sox_executable=sox_executable,
                input_path=input_path,
                output_dir=output_dir,
                sample_rate=args.sample_rate,
                input_channels=BENCHMARK_INPUT_CHANNELS,
                iterations=args.iterations,
                warmups=args.warmups,
            )
        )
    report = {
        "schema": BENCHMARK_REPORT_SCHEMA,
        "generated_at_utc": datetime.now(UTC).replace(microsecond=0).isoformat(),
        "auralis_version": auralis_version_text,
        "sox_ng_version": sox_version_text,
        "config": {
            "repo_root": str(repo_root),
            "output_dir": str(output_dir),
            "input_path": str(input_path),
            "sample_rate": args.sample_rate,
            "duration_seconds": args.duration_seconds,
            "input_channels": BENCHMARK_INPUT_CHANNELS,
            "input_frames": args.sample_rate * args.duration_seconds,
            "iterations": args.iterations,
            "warmups": args.warmups,
            "auralis_binary": str(auralis_bin),
            "build_command": list(RELEASE_BUILD_COMMAND),
            "resume_enabled": bool(args.resume),
        },
        "cases": case_reports,
        "summary": build_report_summary(case_reports),
    }
    return report


def load_resume_cases(
    report_path: Path,
    *,
    repo_root: Path,
    output_dir: Path,
    input_path: Path,
    duration_seconds: int,
    sample_rate: int,
    iterations: int,
    warmups: int,
    auralis_version_text: str,
    sox_version_text: str,
    selected_effects: set[str],
) -> dict[str, dict[str, Any]]:
    """Reuse successful cases from a config-compatible prior report."""

    if not report_path.is_file():
        return {}

    report = json.loads(report_path.read_text(encoding="utf-8"))
    if report.get("schema") != BENCHMARK_REPORT_SCHEMA:
        raise ValueError(f"cannot resume from incompatible report schema: {report_path}")
    if report.get("auralis_version") != auralis_version_text:
        raise ValueError(
            "cannot resume benchmark report with a different Auralis version; rerun without --resume"
        )
    if report.get("sox_ng_version") != sox_version_text:
        raise ValueError(
            "cannot resume benchmark report with a different SoX-ng version; rerun without --resume"
        )

    config = report.get("config", {})
    expected_config = {
        "repo_root": str(repo_root),
        "output_dir": str(output_dir),
        "input_path": str(input_path),
        "sample_rate": sample_rate,
        "duration_seconds": duration_seconds,
        "iterations": iterations,
        "warmups": warmups,
    }
    for key, expected in expected_config.items():
        if config.get(key) != expected:
            raise ValueError(
                f"cannot resume benchmark report with different {key}: {config.get(key)!r} != {expected!r}"
            )

    resumed_cases: dict[str, dict[str, Any]] = {}
    for case in report.get("cases", []):
        if case.get("status") != "ok":
            continue
        effect_name = case.get("effect_name")
        case_id = case.get("case_id")
        if not isinstance(effect_name, str) or not isinstance(case_id, str):
            continue
        if effect_name not in selected_effects:
            continue
        resumed_case = dict(case)
        resumed_case["result_source"] = "reused"
        resumed_cases[case_id] = resumed_case
    return resumed_cases


def benchmark_case(
    case: BenchmarkCase,
    *,
    repo_root: Path,
    auralis_bin: Path,
    sox_executable: str,
    input_path: Path,
    output_dir: Path,
    sample_rate: int,
    input_channels: int,
    iterations: int,
    warmups: int,
) -> dict[str, Any]:
    """Run one benchmark case across all applicable tool/backend variants."""

    skip_reason = should_skip_case(case, sample_rate=sample_rate, input_channels=input_channels)
    if skip_reason is not None:
        return {
            "case_id": case.case_id,
            "effect_name": case.effect_name,
            "tokens": list(case.resolve_tokens()),
            "backend_mode": case.backend_mode.value,
            "token_source": case.token_source,
            "status": "skipped",
            "skip_reason": skip_reason,
            "runs": {},
            "comparisons": {},
        }

    case_dir = output_dir / case.case_id
    case_dir.mkdir(parents=True, exist_ok=True)
    profile_path = case_dir / PROFILE_PLACEHOLDER if case.needs_profile else None
    tokens = case.resolve_tokens(profile_path)

    preparation_error = prepare_case_assets(
        case,
        sox_executable=sox_executable,
        input_path=input_path,
        case_dir=case_dir,
        profile_path=profile_path,
    )
    if preparation_error is not None:
        return {
            "case_id": case.case_id,
            "effect_name": case.effect_name,
            "tokens": list(tokens),
            "backend_mode": case.backend_mode.value,
            "token_source": case.token_source,
            "status": "preparation_failed",
            "preparation_error": preparation_error,
        }

    runs: dict[str, Any] = {}
    runs["sox_ng"] = time_command(
        build_sox_command(sox_executable, input_path, case_dir / "sox_ng.wav", tokens),
        cwd=repo_root,
        iterations=iterations,
        warmups=warmups,
        cleanup_paths=(case_dir / "sox_ng.wav",),
    )
    runs["auralis_scalar"] = time_command(
        build_auralis_command(
            auralis_bin,
            input_path,
            case_dir / "auralis_scalar.wav",
            "scalar",
            tokens,
        ),
        cwd=repo_root,
        iterations=iterations,
        warmups=warmups,
        cleanup_paths=(case_dir / "auralis_scalar.wav",),
    )

    if case.backend_mode is BackendMode.SCALAR_AND_SIMD:
        runs["auralis_simd"] = time_command(
            build_auralis_command(
                auralis_bin,
                input_path,
                case_dir / "auralis_simd.wav",
                "simd",
                tokens,
            ),
            cwd=repo_root,
            iterations=iterations,
            warmups=warmups,
            cleanup_paths=(case_dir / "auralis_simd.wav",),
        )
    else:
        runs["auralis_simd"] = {
            "status": "not_applicable",
            "reason": "effect does not route through a distinct SIMD-aware stage in the current implementation",
        }

    failed_run_keys = failed_required_run_keys(case, runs)
    report = {
        "case_id": case.case_id,
        "effect_name": case.effect_name,
        "tokens": list(tokens),
        "backend_mode": case.backend_mode.value,
        "token_source": case.token_source,
        "status": "failed" if failed_run_keys else "ok",
        "result_source": "measured",
        "runs": runs,
        "comparisons": build_comparisons(runs),
    }
    if failed_run_keys:
        report["failed_runs"] = failed_run_keys
    return report


def should_skip_case(
    case: BenchmarkCase, *, sample_rate: int, input_channels: int
) -> str | None:
    """Return a skip reason when a case is incompatible with benchmark shape constraints."""

    if case.effect_name != "earwax":
        return None
    reasons: list[str] = []
    if sample_rate != EARWAX_REQUIRED_SAMPLE_RATE:
        reasons.append(f"requires sample rate {EARWAX_REQUIRED_SAMPLE_RATE} Hz")
    if input_channels != EARWAX_REQUIRED_CHANNELS:
        reasons.append(f"requires {EARWAX_REQUIRED_CHANNELS} channels")
    if reasons:
        return f"auto-skipped because {case.effect_name} {', '.join(reasons)}; got {sample_rate} Hz, {input_channels} channel(s)"
    return None


def prepare_case_assets(
    case: BenchmarkCase,
    *,
    sox_executable: str,
    input_path: Path,
    case_dir: Path,
    profile_path: Path | None,
) -> str | None:
    """Prepare shared assets required by one case and return an error if needed."""

    if case.effect_name != "noisered":
        return None
    if profile_path is None:
        return "noisered benchmark case is missing a resolved profile path"

    tokens = ("noiseprof", str(profile_path))
    profile_output = case_dir / "profile_seed.wav"
    try:
        subprocess.run(
            build_sox_command(sox_executable, input_path, profile_output, tokens),
            check=True,
            capture_output=True,
            cwd=case_dir,
        )
    except (OSError, subprocess.CalledProcessError) as error:
        return command_error_text(error)
    finally:
        profile_output.unlink(missing_ok=True)

    if not profile_path.is_file():
        return f"expected SoX-ng to produce {profile_path}"
    return None


def build_sox_command(
    sox_executable: str,
    input_path: Path,
    output_path: Path,
    tokens: tuple[str, ...],
) -> list[str]:
    """Build one deterministic SoX-ng benchmark command."""

    return [
        sox_executable,
        "-R",
        "-D",
        str(input_path),
        "-b",
        "16",
        "-e",
        "signed-integer",
        str(output_path),
        *tokens,
    ]


def build_auralis_command(
    auralis_bin: Path,
    input_path: Path,
    output_path: Path,
    backend: str,
    tokens: tuple[str, ...],
) -> list[str]:
    """Build one Auralis benchmark command."""

    return [
        str(auralis_bin),
        "render",
        str(input_path),
        "-o",
        str(output_path),
        "--backend",
        backend,
        "--fx",
        shlex.join(tokens),
    ]


def time_command(
    command: list[str],
    *,
    cwd: Path,
    iterations: int,
    warmups: int,
    cleanup_paths: tuple[Path, ...],
) -> dict[str, Any]:
    """Measure one subprocess command repeatedly."""

    try:
        for _ in range(warmups):
            run_once(command, cwd=cwd, cleanup_paths=cleanup_paths)
        timings_ms = [
            run_once(command, cwd=cwd, cleanup_paths=cleanup_paths) * 1000.0
            for _ in range(iterations)
        ]
    except (OSError, subprocess.CalledProcessError) as error:
        return {
            "status": "failed",
            "command": command,
            "error": command_error_text(error),
        }

    return {
        "status": "ok",
        "command": command,
        "timings_ms": [round(duration, 3) for duration in timings_ms],
        "summary": summarize_timings(timings_ms),
    }


def run_once(command: list[str], *, cwd: Path, cleanup_paths: tuple[Path, ...]) -> float:
    """Run one subprocess benchmark iteration and return elapsed seconds."""

    for path in cleanup_paths:
        path.unlink(missing_ok=True)
    started = time.perf_counter()
    subprocess.run(command, check=True, capture_output=True, cwd=cwd)
    finished = time.perf_counter()
    return finished - started


def summarize_timings(timings_ms: list[float]) -> dict[str, float | str]:
    """Summarize benchmark timings as JSON-safe statistics."""

    mean_ms = statistics.fmean(timings_ms)
    median_ms = statistics.median(timings_ms)
    stdev_ms = statistics.stdev(timings_ms) if len(timings_ms) > 1 else 0.0
    return {
        "min_ms": round(min(timings_ms), 3),
        "max_ms": round(max(timings_ms), 3),
        "mean_ms": round(mean_ms, 3),
        "median_ms": round(median_ms, 3),
        "stdev_ms": round(stdev_ms, 3),
    }


def build_comparisons(runs: dict[str, Any]) -> dict[str, Any]:
    """Build ratio-style comparisons between successful runs."""

    comparisons = {
        "scalar_vs_sox_ng": speed_ratio(runs.get("auralis_scalar"), runs.get("sox_ng")),
        "simd_vs_sox_ng": speed_ratio(runs.get("auralis_simd"), runs.get("sox_ng")),
        "simd_vs_scalar": speed_ratio(runs.get("auralis_simd"), runs.get("auralis_scalar")),
    }
    return comparisons


def failed_required_run_keys(case: BenchmarkCase, runs: dict[str, Any]) -> list[str]:
    """Return failed or missing run keys that make a benchmark case incomplete."""

    required_keys = ["sox_ng", "auralis_scalar"]
    if case.backend_mode is BackendMode.SCALAR_AND_SIMD:
        required_keys.append("auralis_simd")

    failed_keys = []
    for key in required_keys:
        run = runs.get(key)
        if not isinstance(run, dict) or run.get("status") != "ok":
            failed_keys.append(key)
    return failed_keys


def speed_ratio(lhs: dict[str, Any] | None, rhs: dict[str, Any] | None) -> dict[str, Any] | None:
    """Return a median-time ratio between two successful run payloads."""

    if lhs is None or rhs is None:
        return None
    if lhs.get("status") != "ok" or rhs.get("status") != "ok":
        return None
    lhs_median = float(lhs["summary"]["median_ms"])
    rhs_median = float(rhs["summary"]["median_ms"])
    if rhs_median == 0.0:
        return None
    ratio = lhs_median / rhs_median
    return {
        "median_ratio": round(ratio, 3),
        "interpretation": (
            "faster" if ratio < 1.0 else "slower" if ratio > 1.0 else "equal"
        ),
        "speedup": json_number(round(1.0 / ratio, 3)) if ratio > 0.0 else "inf",
    }


def ensure_release_auralis(repo_root: Path, *, skip_build: bool) -> Path:
    """Ensure the release CLI benchmark binary exists and return its path."""

    binary = repo_root / "target/release/auralis"
    if skip_build:
        if not binary.is_file():
            raise FileNotFoundError(
                f"{binary} does not exist; run {' '.join(RELEASE_BUILD_COMMAND)} first or omit --skip-build"
            )
        return binary

    subprocess.run(RELEASE_BUILD_COMMAND, check=True, cwd=repo_root)
    if not binary.is_file():
        raise FileNotFoundError(f"release build completed without producing {binary}")
    return binary


def write_benchmark_input(path: Path, *, duration_seconds: int, sample_rate: int) -> None:
    """Write the shared long-form stereo benchmark input as PCM16 WAV."""

    frames = duration_seconds * sample_rate
    time_axis = np.arange(frames, dtype=np.float64) / float(sample_rate)
    rng = np.random.default_rng(0xA11A11)
    left = (
        0.36 * np.sin(2.0 * math.pi * 220.0 * time_axis)
        + 0.18 * np.sin(2.0 * math.pi * 880.0 * time_axis + 0.3)
        + 0.08 * np.sin(2.0 * math.pi * 6_000.0 * time_axis + 0.1)
        + 0.05 * rng.uniform(-1.0, 1.0, size=frames)
    )
    right = (
        0.33 * np.sin(2.0 * math.pi * 330.0 * time_axis + 0.2)
        + 0.16 * np.sin(2.0 * math.pi * 1_760.0 * time_axis + 0.7)
        + 0.07 * np.sin(2.0 * math.pi * 4_100.0 * time_axis)
        + 0.05 * rng.uniform(-1.0, 1.0, size=frames)
    )
    samples = np.stack(
        [
            np.clip(left, -0.95, 0.95).astype(np.float32),
            np.clip(right, -0.95, 0.95).astype(np.float32),
        ]
    )
    pcm16_fixture(path, samples, sample_rate=sample_rate)


def write_report_files(report: dict[str, Any], output_dir: Path) -> tuple[Path, Path]:
    """Write deterministic JSON and Markdown benchmark reports."""

    json_path = output_dir / "report.json"
    markdown_path = output_dir / "report.md"
    json_path.write_text(json.dumps(report, indent=2, sort_keys=True), encoding="utf-8")
    markdown_path.write_text(render_markdown_report(report), encoding="utf-8")
    return json_path, markdown_path


def build_report_summary(cases: list[dict[str, Any]]) -> dict[str, Any]:
    """Aggregate report-level outcome and performance counts."""

    summary: dict[str, Any] = {
        "total_cases": len(cases),
        "ok_cases": 0,
        "failed_cases": 0,
        "skipped_cases": 0,
        "preparation_failed_cases": 0,
        "reused_cases": 0,
        "scalar_faster_than_sox_ng": 0,
        "scalar_slower_than_sox_ng": 0,
        "scalar_equal_to_sox_ng": 0,
        "simd_faster_than_sox_ng": 0,
        "simd_slower_than_sox_ng": 0,
        "simd_equal_to_sox_ng": 0,
        "simd_not_applicable_cases": 0,
        "scalar_vs_sox_ng_geomean": None,
        "simd_vs_sox_ng_geomean": None,
        "fastest_scalar_vs_sox_ng": None,
        "fastest_simd_vs_sox_ng": None,
        "slowest_scalar_vs_sox_ng": None,
        "slowest_simd_vs_sox_ng": None,
    }

    fastest_scalar: tuple[float, str] | None = None
    fastest_simd: tuple[float, str] | None = None
    slowest_scalar: tuple[float, str] | None = None
    slowest_simd: tuple[float, str] | None = None
    scalar_ratios: list[float] = []
    simd_ratios: list[float] = []

    for case in cases:
        status = case.get("status")
        if status == "ok":
            summary["ok_cases"] += 1
            if case.get("result_source") == "reused":
                summary["reused_cases"] += 1
        elif status == "skipped":
            summary["skipped_cases"] += 1
            continue
        elif status == "preparation_failed":
            summary["failed_cases"] += 1
            summary["preparation_failed_cases"] += 1
            continue
        else:
            summary["failed_cases"] += 1
            continue

        runs = case.get("runs", {})
        if runs.get("auralis_simd", {}).get("status") == "not_applicable":
            summary["simd_not_applicable_cases"] += 1

        scalar_ratio = case.get("comparisons", {}).get("scalar_vs_sox_ng")
        scalar_key = ratio_summary_key("scalar", scalar_ratio)
        if scalar_key is not None:
            summary[scalar_key] += 1
            fastest_scalar = lower_ratio_case(fastest_scalar, scalar_ratio, case["effect_name"])
            slowest_scalar = higher_ratio_case(slowest_scalar, scalar_ratio, case["effect_name"])
            append_positive_ratio(scalar_ratios, scalar_ratio)

        simd_ratio = case.get("comparisons", {}).get("simd_vs_sox_ng")
        simd_key = ratio_summary_key("simd", simd_ratio)
        if simd_key is not None:
            summary[simd_key] += 1
            fastest_simd = lower_ratio_case(fastest_simd, simd_ratio, case["effect_name"])
            slowest_simd = higher_ratio_case(slowest_simd, simd_ratio, case["effect_name"])
            append_positive_ratio(simd_ratios, simd_ratio)

    summary["scalar_vs_sox_ng_geomean"] = ratio_geomean_payload(scalar_ratios)
    summary["simd_vs_sox_ng_geomean"] = ratio_geomean_payload(simd_ratios)
    summary["fastest_scalar_vs_sox_ng"] = fastest_case_payload(fastest_scalar)
    summary["fastest_simd_vs_sox_ng"] = fastest_case_payload(fastest_simd)
    summary["slowest_scalar_vs_sox_ng"] = fastest_case_payload(slowest_scalar)
    summary["slowest_simd_vs_sox_ng"] = fastest_case_payload(slowest_simd)
    return summary


def ratio_summary_key(prefix: str, comparison: dict[str, Any] | None) -> str | None:
    """Return the report summary key for a ratio comparison."""

    if comparison is None:
        return None
    interpretation = comparison.get("interpretation")
    if interpretation is None:
        ratio = comparison.get("median_ratio")
        if isinstance(ratio, int | float):
            if ratio < 1.0:
                interpretation = "faster"
            elif ratio > 1.0:
                interpretation = "slower"
            else:
                interpretation = "equal"
    if interpretation == "faster":
        return f"{prefix}_faster_than_sox_ng"
    if interpretation == "slower":
        return f"{prefix}_slower_than_sox_ng"
    if interpretation == "equal":
        return f"{prefix}_equal_to_sox_ng"
    return None


def lower_ratio_case(
    current: tuple[float, str] | None,
    comparison: dict[str, Any],
    effect_name: str,
) -> tuple[float, str] | None:
    """Keep the case with the lowest successful ratio."""

    ratio = comparison.get("median_ratio")
    if not isinstance(ratio, int | float):
        return current
    candidate = (float(ratio), effect_name)
    if current is None or candidate[0] < current[0]:
        return candidate
    return current


def higher_ratio_case(
    current: tuple[float, str] | None,
    comparison: dict[str, Any],
    effect_name: str,
) -> tuple[float, str] | None:
    """Keep the case with the highest successful ratio."""

    ratio = comparison.get("median_ratio")
    if not isinstance(ratio, int | float):
        return current
    candidate = (float(ratio), effect_name)
    if current is None or candidate[0] > current[0]:
        return candidate
    return current


def append_positive_ratio(ratios: list[float], comparison: dict[str, Any]) -> None:
    """Collect a positive median ratio from a successful comparison."""

    ratio = comparison.get("median_ratio")
    if isinstance(ratio, int | float) and ratio > 0.0:
        ratios.append(float(ratio))


def ratio_geomean_payload(ratios: list[float]) -> dict[str, Any] | None:
    """Return a geometric-mean ratio payload for completed comparisons."""

    if not ratios:
        return None
    ratio = math.prod(ratios) ** (1.0 / len(ratios))
    rounded_ratio = round(ratio, 3)
    return {
        "case_count": len(ratios),
        "median_ratio": rounded_ratio,
        "speedup": json_number(round(1.0 / ratio, 3)),
    }


def fastest_case_payload(case: tuple[float, str] | None) -> dict[str, Any] | None:
    """Render a fastest-case tuple into JSON-safe report data."""

    if case is None:
        return None
    ratio, effect_name = case
    return {
        "effect_name": effect_name,
        "median_ratio": round(ratio, 3),
        "speedup": json_number(round(1.0 / ratio, 3)) if ratio > 0.0 else "inf",
    }


def render_markdown_report(report: dict[str, Any]) -> str:
    """Render the benchmark report as a compact Markdown summary."""

    summary = report.get("summary") or build_report_summary(report["cases"])
    lines = [
        "# SoX-ng Benchmark Report",
        "",
        f"- Generated: {report['generated_at_utc']}",
        f"- Auralis version: {report['auralis_version']}",
        f"- SoX-ng version: {report['sox_ng_version']}",
        f"- Input: {report['config']['duration_seconds']}s @ {report['config']['sample_rate']} Hz, "
        f"{report['config'].get('input_channels', BENCHMARK_INPUT_CHANNELS)} channels, "
        f"{report['config'].get('input_frames', report['config']['sample_rate'] * report['config']['duration_seconds'])} frames",
        f"- Iterations: {report['config']['iterations']} measured, {report['config']['warmups']} warmup",
        f"- Cases: {summary['ok_cases']}/{summary['total_cases']} completed successfully ({summary['skipped_cases']} skipped, {summary['failed_cases']} failed)",
        f"- Reused completed cases: {summary['reused_cases']}",
        f"- Scalar vs SoX-ng: {summary['scalar_faster_than_sox_ng']} faster, {summary['scalar_slower_than_sox_ng']} slower, {summary['scalar_equal_to_sox_ng']} equal",
        f"- SIMD vs SoX-ng: {summary['simd_faster_than_sox_ng']} faster, {summary['simd_slower_than_sox_ng']} slower, {summary['simd_equal_to_sox_ng']} equal, {summary['simd_not_applicable_cases']} not applicable",
        "",
    ]
    scalar_best = summary.get("fastest_scalar_vs_sox_ng")
    scalar_geomean = summary.get("scalar_vs_sox_ng_geomean")
    if scalar_geomean is not None:
        lines.append(
            "- Scalar geometric mean vs SoX-ng: "
            f"{scalar_geomean['median_ratio']} ratio ({scalar_geomean['speedup']}x), "
            f"{scalar_geomean['case_count']} cases"
        )
    if scalar_best is not None:
        lines.append(
            f"- Best scalar speedup vs SoX-ng: {scalar_best['effect_name']} ({scalar_best['speedup']}x, ratio {scalar_best['median_ratio']})"
        )
    scalar_slowest = summary.get("slowest_scalar_vs_sox_ng")
    if scalar_slowest is not None:
        lines.append(
            f"- Highest scalar ratio vs SoX-ng: {scalar_slowest['effect_name']} ({scalar_slowest['speedup']}x, ratio {scalar_slowest['median_ratio']})"
        )
    simd_best = summary.get("fastest_simd_vs_sox_ng")
    simd_geomean = summary.get("simd_vs_sox_ng_geomean")
    if simd_geomean is not None:
        lines.append(
            "- SIMD geometric mean vs SoX-ng: "
            f"{simd_geomean['median_ratio']} ratio ({simd_geomean['speedup']}x), "
            f"{simd_geomean['case_count']} cases"
        )
    if simd_best is not None:
        lines.append(
            f"- Best SIMD speedup vs SoX-ng: {simd_best['effect_name']} ({simd_best['speedup']}x, ratio {simd_best['median_ratio']})"
        )
    simd_slowest = summary.get("slowest_simd_vs_sox_ng")
    if simd_slowest is not None:
        lines.append(
            f"- Highest SIMD ratio vs SoX-ng: {simd_slowest['effect_name']} ({simd_slowest['speedup']}x, ratio {simd_slowest['median_ratio']})"
        )
    lines.extend(
        [
            "",
            "| Effect | Backend Mode | SoX median ms | Scalar median ms | SIMD median ms | Scalar/SoX | SIMD/SoX | SIMD/Scalar |",
            "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )

    for case in report["cases"]:
        status = case.get("status")
        if status == "skipped":
            lines.append(
                f"| {case['effect_name']} | {case['backend_mode']} | skipped | skipped | skipped | n/a | n/a | n/a |"
            )
            continue
        if status != "ok":
            lines.append(
                f"| {case['effect_name']} | {case['backend_mode']} | failed | failed | failed | n/a | n/a | n/a |"
            )
            continue
        runs = case["runs"]
        comparisons = case["comparisons"]
        lines.append(
            "| {effect} | {mode} | {sox} | {scalar} | {simd} | {scalar_vs_sox} | {simd_vs_sox} | {simd_vs_scalar} |".format(
                effect=case["effect_name"],
                mode=case["backend_mode"],
                sox=median_cell(runs.get("sox_ng")),
                scalar=median_cell(runs.get("auralis_scalar")),
                simd=median_cell(runs.get("auralis_simd")),
                scalar_vs_sox=ratio_cell(comparisons.get("scalar_vs_sox_ng")),
                simd_vs_sox=ratio_cell(comparisons.get("simd_vs_sox_ng")),
                simd_vs_scalar=ratio_cell(comparisons.get("simd_vs_scalar")),
            )
        )

    return "\n".join(lines) + "\n"


def median_cell(run: dict[str, Any] | None) -> str:
    """Return a Markdown table cell for a run median."""

    if run is None:
        return "n/a"
    if run.get("status") == "not_applicable":
        return "n/a"
    if run.get("status") != "ok":
        return "failed"
    return str(run["summary"]["median_ms"])


def ratio_cell(comparison: dict[str, Any] | None) -> str:
    """Return a Markdown table cell for a ratio comparison."""

    if comparison is None:
        return "n/a"
    return str(comparison["median_ratio"])


def command_error_text(error: OSError | subprocess.CalledProcessError) -> str:
    """Render a concise subprocess failure string."""

    if isinstance(error, OSError):
        return str(error)
    stderr = error.stderr.decode("utf-8", errors="replace").strip() if error.stderr else ""
    stdout = error.stdout.decode("utf-8", errors="replace").strip() if error.stdout else ""
    detail = stderr or stdout
    if detail:
        return f"exit code {error.returncode}: {detail}"
    return f"exit code {error.returncode}"


def positive_int(value: str) -> int:
    """Argparse type enforcing a positive integer."""

    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("value must be positive")
    return parsed


def benchmark_duration_seconds(value: str) -> int:
    """Argparse type enforcing a long enough benchmark input duration."""

    parsed = positive_int(value)
    if parsed < MIN_DURATION_SECONDS:
        raise argparse.ArgumentTypeError(
            f"value must be at least {MIN_DURATION_SECONDS} seconds to avoid startup-dominated timings"
        )
    return parsed


def nonnegative_int(value: str) -> int:
    """Argparse type enforcing a non-negative integer."""

    parsed = int(value)
    if parsed < 0:
        raise argparse.ArgumentTypeError("value must be non-negative")
    return parsed


def list_cases(repo_root: Path) -> str:
    """Render the benchmark case catalog as plain text."""

    lines = []
    for case in benchmark_case_catalog(repo_root):
        lines.append(
            f"{case.effect_name}\t{case.backend_mode.value}\t{case.token_source}\t{' '.join(case.tokens)}"
        )
    return "\n".join(lines) + ("\n" if lines else "")


def main(argv: list[str] | None = None) -> int:
    """Run the benchmark CLI entrypoint."""

    args = parse_args(argv)
    repo_root = args.repo_root.resolve()
    if args.list_cases:
        print(list_cases(repo_root), end="")
        return 0

    report = run_benchmarks(args)
    output_dir = (repo_root / args.output_dir).resolve()
    json_path, markdown_path = write_report_files(report, output_dir)
    print(f"wrote {json_path}")
    print(f"wrote {markdown_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
