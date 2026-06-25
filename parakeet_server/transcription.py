"""
Parakeet-TDT-0.6B-v3 transcription pipeline — multilingue (style POC).

Model card: https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3
Architecture: FastConformer-Transducer (CTC/Transducer hybrid)

Supported languages (ISO 639-3):
  Bulgarian (bul), Croatian (hrv), Czech (ces), Danish (dan), Dutch (nld),
  English (eng), Estonian (est), Finnish (fin), French (fra), German (deu),
  Greek (ell), Hungarian (hun), Italian (ita), Latvian (lav), Lithuanian (lit),
  Maltese (mlt), Polish (pol), Portuguese (por), Romanian (ron), Slovak (slk),
  Slovenian (slv), Spanish (spa), Swedish (swe), Russian (rus), Ukrainian (ukr)
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

# ISO 639-1 → ISO 639-3 (Parakeet-TDT-v3 languages)
ISO_639_1_TO_3 = {
    "en": "eng", "fr": "fra", "es": "spa", "de": "deu", "it": "ita",
    "pt": "por", "nl": "nld", "pl": "pol", "ru": "rus", "uk": "ukr",
    "cs": "ces", "sk": "slk", "sl": "slv", "hr": "hrv", "bg": "bul",
    "ro": "ron", "hu": "hun", "el": "ell", "da": "dan", "sv": "swe",
    "no": "nor", "fi": "fin", "et": "est", "lv": "lav", "lt": "lit",
    "mt": "mlt", "ca": "cat", "ga": "gle",
}

# In TDT output <blank> marks word boundaries (not silence), so it must become a space.
_BLANK_RE = re.compile(r"<blank>")

DetectorFactory.seed = 0  # deterministic langdetect

MODEL_NAME = "nvidia/parakeet-tdt-0.6b-v3"


class ParakeetTranscriber:
    """Thread-safe wrapper around the Parakeet-TDT ASR pipeline (multilingue)."""

    def __init__(
        self,
        model_name: str = MODEL_NAME,
        device: str = "cpu",
    ):
        self.model_name = model_name
        self.device = device
        self._processor = None
        self._model = None
        self._loading = False
        self._loaded = False
        self._load_error: Optional[str] = None
        self.torch_version = torch.__version__

    def is_loaded(self) -> bool:
        return self._loaded and self._model is not None

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

            # Warm-up
            dummy = np.zeros(16000, dtype=np.float32)
            _ = self._processor(dummy, sampling_rate=16000, return_tensors="pt")
            _ = self._model.generate(**_)

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
        Transcribe WAV audio bytes (16kHz mono float32).

        Args:
            audio_bytes: Raw WAV file bytes
            language: Ignored — language is auto-detected from transcription

        Returns:
            dict with keys: text, language, segments, duration_seconds, inference_time_ms
        """
        if not self._loaded or self._model is None:
            raise RuntimeError("Model not loaded")

        try:
            audio_io = io.BytesIO(audio_bytes)
            audio_array, sample_rate = sf.read(audio_io, dtype="float32")
        except Exception as e:
            raise ValueError(f"Failed to decode WAV: {e}")

        if len(audio_array.shape) > 1:
            audio_array = audio_array[:, 0]

        if sample_rate != 16000:
            raise ValueError(f"Expected 16kHz audio, got {sample_rate}Hz")

        if audio_array.dtype != np.float32:
            audio_array = audio_array.astype(np.float32)

        duration = len(audio_array) / 16000.0

        if duration < 0.1:
            raise ValueError("Audio too short (< 0.1s)")

        start_time = time.perf_counter()

        inputs = self._processor(audio_array, sampling_rate=16000, return_tensors="pt")
        with torch.no_grad():
            output = self._model.generate(**inputs)

        inference_time = (time.perf_counter() - start_time) * 1000

        transcription = self._processor.batch_decode(output.sequences)[0]
        # Postprocessing: <blank> → espace, collapse spaces, strip, lowercase
        cleaned = _BLANK_RE.sub(" ", transcription)
        cleaned = re.sub(r"\s+", " ", cleaned).strip()
        text = cleaned.lower()

        # Language detection via langdetect on cleaned transcription
        detected_lang = "und"
        if len(text) > 20:
            try:
                candidates = detect_langs(text)
                top = candidates[0]
                if top.prob > 0.5:
                    lang_639_1 = top.lang
                    detected_lang = ISO_639_1_TO_3.get(lang_639_1, lang_639_1)
            except Exception:
                pass

        return {
            "text": text,
            "language": detected_lang,
            "segments": [],
            "duration_seconds": round(duration, 2),
            "inference_time_ms": round(inference_time, 1),
        }
