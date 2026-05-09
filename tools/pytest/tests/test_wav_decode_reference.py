from __future__ import annotations

import subprocess
from os import environ
from pathlib import Path
from shutil import which

import numpy as np
import pytest
from scipy.io import wavfile


def test_pcm16_decode_matches_python_reference(tmp_path: Path) -> None:
    repo_root = Path(__file__).resolve().parents[3]
    wav_path = tmp_path / "reference.wav"
    interleaved = np.array(
        [[-32768, 32767], [-16384, 16384], [0, 8192]],
        dtype=np.int16,
    )
    wavfile.write(wav_path, 48_000, interleaved)

    decoded = subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "--package",
            "auralis-wav",
            "--example",
            "decode_pcm16",
            "--",
            str(wav_path),
        ],
        cwd=repo_root,
        check=True,
        text=True,
        capture_output=True,
    )

    fields = dict(line.split("=", 1) for line in decoded.stdout.strip().splitlines())
    planar = np.fromstring(fields["planar"], sep=",", dtype=np.float32)
    expected = (interleaved.astype(np.float32) / np.float32(32768.0)).T.reshape(-1)

    assert fields["sample_rate"] == "48000"
    assert fields["channels"] == "2"
    assert fields["frames"] == "3"
    np.testing.assert_array_equal(planar, expected)


@pytest.mark.golden
def test_cli_gain_matches_sox_ng_golden(tmp_path: Path) -> None:
    repo_root = Path(__file__).resolve().parents[3]
    sox_ng = environ.get("AURALIS_SOX_NG_BIN") or which("sox_ng")
    if sox_ng is None:
        pytest.skip("sox_ng is not available in PATH")

    input_path = tmp_path / "input.wav"
    auralis_output = tmp_path / "auralis.wav"
    sox_output = tmp_path / "sox.wav"
    samples = np.array(
        [[-20_000, 20_000], [-12_000, 12_000], [0, 4096], [16_000, -16_000]],
        dtype=np.int16,
    )
    wavfile.write(input_path, 48_000, samples)

    subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "--package",
            "auralis-cli",
            "--",
            "run",
            str(input_path),
            str(auralis_output),
            "--gain-db",
            "-3",
        ],
        cwd=repo_root,
        check=True,
    )
    subprocess.run(
        [
            sox_ng,
            "-R",
            "-D",
            str(input_path),
            "-b",
            "16",
            "-e",
            "signed-integer",
            str(sox_output),
            "gain",
            "-3",
        ],
        check=True,
    )

    auralis_rate, auralis_samples = wavfile.read(auralis_output)
    sox_rate, sox_samples = wavfile.read(sox_output)

    assert auralis_rate == sox_rate == 48_000
    np.testing.assert_allclose(
        auralis_samples.astype(np.int32),
        sox_samples.astype(np.int32),
        atol=1,
        rtol=0,
    )


@pytest.mark.golden
def test_cli_trim_matches_sox_ng_golden(tmp_path: Path) -> None:
    repo_root = Path(__file__).resolve().parents[3]
    sox_ng = environ.get("AURALIS_SOX_NG_BIN") or which("sox_ng")
    if sox_ng is None:
        pytest.skip("sox_ng is not available in PATH")

    input_path = tmp_path / "input.wav"
    auralis_output = tmp_path / "auralis.wav"
    sox_output = tmp_path / "sox.wav"
    samples = np.array(
        [
            [-20_000, 20_000],
            [-12_000, 12_000],
            [-4_000, 4_000],
            [0, 4096],
            [16_000, -16_000],
        ],
        dtype=np.int16,
    )
    wavfile.write(input_path, 48_000, samples)

    subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "--package",
            "auralis-cli",
            "--",
            "run",
            str(input_path),
            str(auralis_output),
            "--trim-start-frame",
            "1",
            "--trim-end-frame",
            "4",
        ],
        cwd=repo_root,
        check=True,
    )
    subprocess.run(
        [
            sox_ng,
            "-R",
            "-D",
            str(input_path),
            "-b",
            "16",
            "-e",
            "signed-integer",
            str(sox_output),
            "trim",
            "0.000020833333333333333",
            "0.0000625",
        ],
        check=True,
    )

    auralis_rate, auralis_samples = wavfile.read(auralis_output)
    sox_rate, sox_samples = wavfile.read(sox_output)

    assert auralis_rate == sox_rate == 48_000
    np.testing.assert_array_equal(auralis_samples, sox_samples)


@pytest.mark.golden
def test_cli_pad_matches_sox_ng_golden(tmp_path: Path) -> None:
    repo_root = Path(__file__).resolve().parents[3]
    sox_ng = environ.get("AURALIS_SOX_NG_BIN") or which("sox_ng")
    if sox_ng is None:
        pytest.skip("sox_ng is not available in PATH")

    input_path = tmp_path / "input.wav"
    auralis_output = tmp_path / "auralis.wav"
    sox_output = tmp_path / "sox.wav"
    samples = np.array(
        [[-20_000, 20_000], [-12_000, 12_000], [0, 4096]],
        dtype=np.int16,
    )
    wavfile.write(input_path, 48_000, samples)

    subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "--package",
            "auralis-cli",
            "--",
            "run",
            str(input_path),
            str(auralis_output),
            "--pad-start-frame",
            "2",
            "--pad-end-frame",
            "1",
        ],
        cwd=repo_root,
        check=True,
    )
    subprocess.run(
        [
            sox_ng,
            "-R",
            "-D",
            str(input_path),
            "-b",
            "16",
            "-e",
            "signed-integer",
            str(sox_output),
            "pad",
            "0.000041666666666666667",
            "0.000020833333333333333",
        ],
        check=True,
    )

    auralis_rate, auralis_samples = wavfile.read(auralis_output)
    sox_rate, sox_samples = wavfile.read(sox_output)

    assert auralis_rate == sox_rate == 48_000
    np.testing.assert_array_equal(auralis_samples, sox_samples)


@pytest.mark.golden
def test_cli_reverse_matches_sox_ng_golden(tmp_path: Path) -> None:
    repo_root = Path(__file__).resolve().parents[3]
    sox_ng = environ.get("AURALIS_SOX_NG_BIN") or which("sox_ng")
    if sox_ng is None:
        pytest.skip("sox_ng is not available in PATH")

    input_path = tmp_path / "input.wav"
    auralis_output = tmp_path / "auralis.wav"
    sox_output = tmp_path / "sox.wav"
    samples = np.array(
        [
            [-20_000, 20_000],
            [-12_000, 12_000],
            [0, 4096],
            [16_000, -16_000],
        ],
        dtype=np.int16,
    )
    wavfile.write(input_path, 48_000, samples)

    subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "--package",
            "auralis-cli",
            "--",
            "run",
            str(input_path),
            str(auralis_output),
            "--reverse",
        ],
        cwd=repo_root,
        check=True,
    )
    subprocess.run(
        [
            sox_ng,
            "-R",
            "-D",
            str(input_path),
            "-b",
            "16",
            "-e",
            "signed-integer",
            str(sox_output),
            "reverse",
        ],
        check=True,
    )

    auralis_rate, auralis_samples = wavfile.read(auralis_output)
    sox_rate, sox_samples = wavfile.read(sox_output)

    assert auralis_rate == sox_rate == 48_000
    np.testing.assert_array_equal(auralis_samples, sox_samples)
