"""
ParakeetSTT POC — Transcription
Called by the Rust binary. Runs Parakeet-TDT-v3 inference on a 16kHz mono WAV.

Usage: python3 transcribe.py <path_to_wav>
Output: JSON {text, language, duration_s}
  - text:      lowercased transcription
  - language:  ISO 639-3 code (e.g. "eng", "fra", "spa"), from langdetect on the transcription ("und" if uncertain)
  - duration_s: input audio length
"""

import sys
import os
import json
import re
from pathlib import Path
import soundfile as sf
import torch
import numpy as np
from transformers import AutoModel, AutoProcessor
from langdetect import DetectorFactory, detect_langs

# In TDT output <blank> marks word boundaries (not silence), so it must become a space.
_BLANK_RE = re.compile(r"<blank>")

DetectorFactory.seed = 0  # deterministic langdetect

MODEL_NAME = "nvidia/parakeet-tdt-0.6b-v3"
CACHE_DIR = Path(__file__).parent / "models"
CACHE_DIR.mkdir(exist_ok=True)
os.environ.setdefault("HF_HOME", str(CACHE_DIR))

# ISO 639-1 → ISO 639-3 for the languages Parakeet-TDT-v3 supports.
ISO_639_1_TO_3 = {
    "en": "eng", "fr": "fra", "es": "spa", "de": "deu", "it": "ita",
    "pt": "por", "nl": "nld", "pl": "pol", "ru": "rus", "uk": "ukr",
    "cs": "ces", "sk": "slk", "sl": "slv", "hr": "hrv", "bg": "bul",
    "ro": "ron", "hu": "hun", "el": "ell", "da": "dan", "sv": "swe",
    "no": "nor", "fi": "fin", "et": "est", "lv": "lav", "lt": "lit",
    "mt": "mlt", "ca": "cat", "ga": "gle",
}

# Load model once at startup (module-level cache)
print(f"Loading {MODEL_NAME} (cache: {CACHE_DIR})...", file=sys.stderr)
processor = AutoProcessor.from_pretrained(MODEL_NAME, cache_dir=CACHE_DIR)
model = AutoModel.from_pretrained(
    MODEL_NAME,
    cache_dir=CACHE_DIR,
    torch_dtype=torch.float32,
    device_map="cpu",
)
model.eval()
print("Model ready.", file=sys.stderr)

# Warm-up
_dummy = np.zeros(16000, dtype=np.float32)
_ = processor(_dummy, sampling_rate=16000, return_tensors="pt")


def main():
    if len(sys.argv) < 2:
        print(json.dumps({"error": "Usage: python3 transcribe.py <wav_path>"}))
        sys.exit(1)

    wav_path = sys.argv[1]

    # Load audio
    audio_array, sr = sf.read(wav_path, dtype="float32")

    if len(audio_array.shape) > 1:
        audio_array = audio_array[:, 0]  # stereo → mono

    if sr != 16000:
        raise ValueError(f"Expected 16kHz audio, got {sr} Hz")

    duration = len(audio_array) / 16000.0
    print(f"Transcribing {duration:.1f}s of audio...", file=sys.stderr)

    # Inference
    inputs = processor(audio_array, sampling_rate=16000, return_tensors="pt")
    with torch.no_grad():
        output = model.generate(**inputs)

    transcription = processor.batch_decode(output.sequences)[0]
    cleaned = _BLANK_RE.sub(" ", transcription)
    cleaned = re.sub(r"\s+", " ", cleaned).strip()

    # Language identification via langdetect on the transcription.
    # Requires >20 chars and top-prob >0.5 to be considered reliable; else "und".
    language = "und"
    if len(cleaned) > 20:
        try:
            candidates = detect_langs(cleaned)
            top = candidates[0]
            if top.prob > 0.5:
                language = ISO_639_1_TO_3.get(top.lang, top.lang)
        except Exception:
            pass

    result = {
        "text": cleaned.lower(),
        "language": language,
        "duration_s": round(duration, 2),
    }

    print(json.dumps(result, ensure_ascii=False))


if __name__ == "__main__":
    main()
