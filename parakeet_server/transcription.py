"""
Parakeet-TDT-0.6B-v3 transcription pipeline wrapper.

Model card: https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3
Architecture: FastConformer-Transducer (CTC/Transducer hybrid)

Supported languages:
  Bulgarian (bg), Croatian (hr), Czech (cs), Danish (da), Dutch (nl),
  English (en), Estonian (et), Finnish (fi), French (fr), German (de),
  Greek (el), Hungarian (hu), Italian (it), Latvian (lv), Lithuanian (lt),
  Maltese (mt), Polish (pl), Portuguese (pt), Romanian (ro), Slovak (sk),
  Slovenian (sl), Spanish (es), Swedish (sv), Russian (ru), Ukrainian (uk)
"""

import io
import time
import logging
from typing import Optional

import numpy as np
import soundfile as sf
import torch
from transformers import AutoModelForCTC, AutoProcessor, pipeline

logger = logging.getLogger("parakeet_transcription")

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
        self._pipeline = None
        self._loading = False
        self._loaded = False
        self._load_error: Optional[str] = None
        self.torch_version = torch.__version__

    def is_loaded(self) -> bool:
        return self._loaded and self._pipeline is not None

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

            processor = AutoProcessor.from_pretrained(self.model_name)
            model = AutoModelForCTC.from_pretrained(
                self.model_name,
                torch_dtype=torch.float32,
                device_map=self.device,
            )
            model.eval()

            self._pipeline = pipeline(
                "automatic-speech-recognition",
                model=model,
                tokenizer=processor.tokenizer,
                feature_extractor=processor,
                torch_dtype=torch.float32,
                device=self.device,
                chunk_length_s=30,
                return_timestamps=True,
            )

            logger.info("Warming up pipeline with silence...")
            dummy_audio = np.zeros(16000, dtype=np.float32)
            _ = self._pipeline(
                {"array": dummy_audio, "sampling_rate": 16000},
                generate_kwargs={"language": "en"},
            )
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
        if not self._loaded or self._pipeline is None:
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

        lang_code = language if language != "auto" else None

        start_time = time.perf_counter()

        result = self._pipeline(
            {"array": audio_array, "sampling_rate": 16000},
            generate_kwargs={"language": lang_code} if lang_code else {},
            return_timestamps=True,
        )

        inference_time = (time.perf_counter() - start_time) * 1000

        text = result.get("text", "").strip()
        detected_lang = result.get("language", language if language != "auto" else "en")

        segments = []
        if "chunks" in result:
            for chunk in result["chunks"]:
                segments.append({
                    "start": chunk.get("timestamp", (0, 0))[0],
                    "end": chunk.get("timestamp", (0, 0))[1],
                    "text": chunk.get("text", "").strip(),
                })

        return {
            "text": text,
            "language": detected_lang,
            "segments": segments,
            "duration_seconds": round(duration, 2),
            "inference_time_ms": round(inference_time, 1),
        }
