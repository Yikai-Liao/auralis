"""Deterministic audio corpus helpers for pytest-based Auralis tests."""

from __future__ import annotations

import math
from pathlib import Path

import numpy as np
from scipy.io import wavfile


def sine_wave(
    *,
    sample_rate: int = 48_000,
    frequency_hz: float = 1_000.0,
    frames: int,
    channels: int = 1,
    amplitude: float = 0.5,
    phase: float = 0.0,
) -> np.ndarray:
    """Return a deterministic planar float32 sine wave."""

    if sample_rate <= 0:
        raise ValueError("sample_rate must be positive")
    if frames < 0:
        raise ValueError("frames must be non-negative")
    if channels <= 0:
        raise ValueError("channels must be positive")
    if not math.isfinite(frequency_hz):
        raise ValueError("frequency_hz must be finite")
    if not math.isfinite(amplitude):
        raise ValueError("amplitude must be finite")
    if not math.isfinite(phase):
        raise ValueError("phase must be finite")

    frame_index = np.arange(frames, dtype=np.float32)
    radians = (
        np.float32(2.0 * math.pi * frequency_hz / sample_rate) * frame_index
        + np.float32(phase)
    )
    mono = np.sin(radians, dtype=np.float32) * np.float32(amplitude)
    return np.tile(mono, (channels, 1))


def pcm16_fixture(path: Path, samples: np.ndarray, *, sample_rate: int = 48_000) -> Path:
    """Write normalized planar float samples as a PCM16 WAV fixture."""

    if sample_rate <= 0:
        raise ValueError("sample_rate must be positive")

    normalized = np.asarray(samples, dtype=np.float32)
    if normalized.ndim == 1:
        normalized = normalized.reshape(1, -1)
    if normalized.ndim != 2:
        raise ValueError("samples must be a mono vector or planar channel matrix")
    if not np.all(np.isfinite(normalized)):
        raise ValueError("samples must be finite")

    quantized = np.rint(np.clip(normalized, -1.0, 1.0) * np.float32(32768.0))
    pcm = np.clip(quantized, -32768, 32767).astype(np.int16)
    interleaved = pcm.T if pcm.shape[0] > 1 else pcm.reshape(-1)
    wavfile.write(path, sample_rate, interleaved)
    return path
