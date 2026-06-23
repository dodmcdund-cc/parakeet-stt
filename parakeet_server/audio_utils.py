"""
Audio utilities for WAV validation and format conversion.
"""

import io
import numpy as np
import soundfile as sf


def validate_wav(audio_bytes: bytes) -> np.ndarray:
    """
    Validate and decode WAV bytes to a normalized float32 numpy array.

    Args:
        audio_bytes: Raw WAV file bytes

    Returns:
        numpy array of shape (n_samples,) dtype float32, normalized to [-1, 1]

    Raises:
        ValueError: If the audio is invalid or doesn't meet requirements
    """
    if len(audio_bytes) < 44:
        raise ValueError(f"File too small ({len(audio_bytes)} bytes) — invalid WAV")

    try:
        audio_io = io.BytesIO(audio_bytes)
        audio_array, sample_rate = sf.read(audio_io, dtype="float32")
    except Exception as e:
        raise ValueError(f"Cannot decode WAV: {e}")

    if sample_rate != 16000:
        raise ValueError(f"Sample rate must be 16000 Hz, got {sample_rate}")

    if len(audio_array.shape) > 1:
        audio_array = audio_array[:, 0]

    if audio_array.dtype != np.float32:
        audio_array = audio_array.astype(np.float32)

    max_val = np.abs(audio_array).max()
    if max_val > 1.0:
        audio_array = audio_array / max_val

    return audio_array


def floats_to_wav(samples: np.ndarray, sample_rate: int = 16000) -> bytes:
    """
    Encode a float32 numpy array as WAV bytes.

    Args:
        samples: numpy array of shape (n_samples,) dtype float32, range [-1, 1]
        sample_rate: sample rate (default 16kHz)

    Returns:
        bytes: RIFF WAV file
    """
    output = io.BytesIO()
    sf.write(output, samples, sample_rate, format="WAV", subtype="FLOAT")
    return output.getvalue()
