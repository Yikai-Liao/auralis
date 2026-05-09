from __future__ import annotations

import subprocess
from pathlib import Path

import numpy as np
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
