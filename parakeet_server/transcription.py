"""
Parakeet-TDT-0.6B-v3 transcription pipeline wrapper.

Model card: https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3
Architecture: FastConformer-Transducer (Transducer)

Supported languages:
  Bulgarian (bg), Croatian (hr), Czech (cs), Danish (da), Dutch (nl),
  English (en), Estonian (et), Finnish (fi), French (fr), German (de),
  Greek (el), Hungarian (hu), Italian (it), Latvian (lv), Lithuanian (lt),
  Maltese (mt), Polish (pl), Portuguese (pt), Romanian (ro), Slovak (sk),
  Slovenian (sl), Spanish (es), Swedish (sv), Russian (ru), Ukrainian (uk)
"""

import io
import re
import time
import logging
from typing import Optional

import numpy as np
import soundfile as sf
import torch
from transformers import AutoModel, AutoProcessor
from langdetect import DetectorFactory, detect_langs

logger = logging.getLogger("parakeet_transcription")

# In TDT output <blank> marks word boundaries, so it must become a space.
_BLANK_RE = re.compile(r"<blank>")

DetectorFactory.seed = 0  # deterministic langdetect

LANGUAGE_MAP = {
    "bg": "bulgarian",
    "hr": "croatian",
    "cs": "czech",
    "da": "danish",
    "nl": "dutch",
    "en": "english",
    "et": "estonian",
    "fi": "finnish",
    "fr": "french",
    "de": "german",
    "el": "greek",
    "hu": "hungarian",
    "it": "italian",
    "lv": "latvian",
    "lt": "lithuanian",
    "mt": "maltese",
    "pl": "polish",
    "pt": "portuguese",
    "ro": "romanian",
    "sk": "slovak",
    "sl": "slovenian",
    "es": "spanish",
    "sv": "swedish",
    "ru": "russian",
    "uk": "ukrainian",
}

SUPPORTED_LANGUAGES = list(LANGUAGE_MAP.keys())


class ParakeetTranscriber:
    """Thread-safe wrapper around the Parakeet-TDT ASR pipeline."""

    def __init__(
        self,
        model_name: str = "nvidia/parakeet-tdt-0.6b-v3",
        device: str = "cpu",
    ):
        self.model_name = model_name
        self.device = device
        self._model = None
        self._processor = None
        self._loading = False
        self._loaded = False
        self._load_error: Optional[str] = None
        self.torch_version = torch.__version__

    def is_loaded(self) -> bool:
        return self._loaded and self._model is not None and self._processor is not None

    def get_status(self) -> dict:
        return {
            "loaded": self._loaded,
            "loading": self._loading,
            "error": self._load_error,
            "model_name": self.model_name,
        }

    async def load(self):
        """Load the Parakeet model into memory (CPU)."""
        if self._loaded or self._loading:
            return

        self._loading = True
        self._load_error = None

        try:
            logger.info(f"Loading {self.model_name} on {self.device}...")

            self._processor = AutoProcessor.from_pretrained(self.model_name)
            self._model = AutoModel.from_pretrained(
                self.model_name,
                torch_dtype=torch.float32,
                device_map=self.device,
            )
            self._model.eval()

            logger.info("Warming up model with silence...")
            dummy = np.zeros(16000, dtype=np.float32)
            inputs = self._processor(dummy, sampling_rate=16000, return_tensors="pt")
            with torch.no_grad():
                _ = self._model.generate(**inputs)
            logger.info("Warm-up complete.")

            self._loaded = True
            logger.info("Model loaded successfully.")

        except Exception as e:
            self._load_error = str(e)
            logger.error(f"Failed to load model: {e}")
            raise

        finally:
            self._loading = False

    async def transcribe(
        self,
        audio_bytes: bytes,
        language: str = "auto",
    ) -> dict:
        """
        Transcribe WAV audio bytes.

        Args:
            audio_bytes: Raw WAV file bytes (16kHz, mono, float32)
            language: ISO 639-1 language code or "auto"

        Returns:
            dict with keys: text, language, segments, duration_seconds, inference_time_ms
        """
        if not self._loaded or self._model is None or self._processor is None:
            raise RuntimeError("Model not loaded")

        try:
            audio_io = io.BytesIO(audio_bytes)
            audio_array, sample_rate = sf.read(audio_io, dtype="float32")
        except Exception as e:
            raise ValueError(f"Failed to decode WAV: {e}")

        if sample_rate != 16000:
            raise ValueError(f"Expected 16kHz audio, got {sample_rate}Hz")
        if len(audio_array.shape) > 1:
            audio_array = audio_array[:, 0]
        if audio_array.dtype != np.float32:
            audio_array = audio_array.astype(np.float32)

        duration = len(audio_array) / 16000.0

        if duration < 0.1:
            raise ValueError("Audio too short (< 0.1s)")

        max_val = np.abs(audio_array).max()
        if max_val > 0:
            audio_array = audio_array / max_val

        start_time = time.perf_counter()

        inputs = self._processor(audio_array, sampling_rate=16000, return_tensors="pt")
        with torch.no_grad():
            output = self._model.generate(**inputs)

        inference_time = (time.perf_counter() - start_time) * 1000

        transcription = self._processor.batch_decode(output.sequences)[0]
        text = _BLANK_RE.sub(" ", transcription)
        text = re.sub(r"\s+", " ", text).strip().lower()

        # Language detection via langdetect on the transcription.
        # Requires >20 chars and top-prob >0.5 to be considered reliable; else "und".
        if language != "auto":
            detected_lang = language
        elif len(text) > 20:
            try:
                candidates = detect_langs(text)
                top = candidates[0]
                if top.prob > 0.5:
                    detected_lang = top.lang  # ISO 639-1 directly
                else:
                    detected_lang = "und"
            except Exception:
                detected_lang = "und"
        else:
            detected_lang = "und"

        return {
            "text": text,
            "language": detected_lang,
            "segments": [],
            "duration_seconds": round(duration, 2),
            "inference_time_ms": round(inference_time, 1),
        }
