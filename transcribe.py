"""
ParakeetSTT POC — Transcription
Called by the Rust binary. Runs Parakeet-TDT-v3 inference on a 16kHz mono WAV.

Usage: python3 transcribe.py <path_to_wav>
Output: JSON {text, language, duration_s}
"""

import sys
import json
import soundfile as sf
import torch
import numpy as np
from transformers import AutoModelForCTC, AutoProcessor

MODEL_NAME = "nvidia/parakeet-tdt-0.6b-v3"

# Load model once at startup (module-level cache)
print(f"Loading {MODEL_NAME}...", file=sys.stderr)
processor = AutoProcessor.from_pretrained(MODEL_NAME)
model = AutoModelForCTC.from_pretrained(
    MODEL_NAME,
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
        logits = model(inputs.input_values).logits

    predicted_ids = torch.argmax(logits, dim=-1)
    transcription = processor.batch_decode(predicted_ids)[0]

    result = {
        "text": transcription.lower().strip(),
        "language": "en",  # Parakeet supports auto-detection; simplified here
        "duration_s": round(duration, 2),
    }

    print(json.dumps(result))


if __name__ == "__main__":
    main()
