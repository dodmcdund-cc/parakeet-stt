"""
ParakeetSTT POC — Transcription
Called by the Rust binary. Runs Parakeet-TDT-v3 inference on a 16kHz mono WAV.

Usage: python3 transcribe.py <path_to_wav>
Output: JSON {text, language, duration_s}
  - text:      lowercased transcription
  - language:  ISO 639-3 code (e.g. "eng", "fra", "spa"), from MMS-LID-256
  - duration_s: input audio length
"""

import sys
import os
import json
from pathlib import Path
import soundfile as sf
import torch
import numpy as np
from transformers import AutoModel, AutoProcessor, Wav2Vec2ForSequenceClassification, AutoFeatureExtractor

MODEL_NAME = "nvidia/parakeet-tdt-0.6b-v3"
LID_MODEL_NAME = "facebook/mms-lid-256"
CACHE_DIR = Path(__file__).parent / "models"
CACHE_DIR.mkdir(exist_ok=True)
os.environ.setdefault("HF_HOME", str(CACHE_DIR))

# Load models once at startup (module-level cache)
print(f"Loading {MODEL_NAME} (cache: {CACHE_DIR})...", file=sys.stderr)
processor = AutoProcessor.from_pretrained(MODEL_NAME, cache_dir=CACHE_DIR)
model = AutoModel.from_pretrained(
    MODEL_NAME,
    cache_dir=CACHE_DIR,
    torch_dtype=torch.float32,
    device_map="cpu",
)
model.eval()

print(f"Loading {LID_MODEL_NAME}...", file=sys.stderr)
lid_feature_extractor = AutoFeatureExtractor.from_pretrained(LID_MODEL_NAME, cache_dir=CACHE_DIR)
lid_model = Wav2Vec2ForSequenceClassification.from_pretrained(LID_MODEL_NAME, cache_dir=CACHE_DIR)
lid_model.eval()
print("Models ready.", file=sys.stderr)

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

    # Language identification (3-letter ISO 639-3 codes, e.g. "eng", "fra")
    lid_inputs = lid_feature_extractor(audio_array, sampling_rate=16000, return_tensors="pt")
    with torch.no_grad():
        lid_logits = lid_model(**lid_inputs).logits
    language = lid_model.config.id2label[torch.argmax(lid_logits, dim=-1).item()]

    result = {
        "text": transcription.lower().strip(),
        "language": language,
        "duration_s": round(duration, 2),
    }

    print(json.dumps(result))


if __name__ == "__main__":
    main()
