# ParakeetSTT — Proof of Concept

Minimal end-to-end demo: record 5 seconds of audio → transcribe via NVIDIA Parakeet-TDT-v3 → print text.

This POC validates the core pipeline before building the full [ParakeetSTT](https://github.com/dodmcdund-cc/parakeet-stt) application.

---

## What it proves

- ✅ Audio capture pipeline (cpal = CoreAudio on macOS, WASAPI on Windows)
- ✅ WAV encoding (16kHz mono Float32)
- ✅ Transformers / Parakeet-v3 inference on CPU
- ✅ Rust ↔ Python interop (subprocess)

---

## Prerequisites

- **Python 3.11+**
- **Rust 1.70+**
- **~2 GB free RAM** (model loaded in CPU memory)

---

## Setup

### 1. Clone the repo

```bash
git clone https://github.com/dodmcdund-cc/parakeet-stt
cd parakeet-stt
```

### 2. Python environment

```bash
# macOS / Linux
python3 -m venv .venv
source .venv/bin/activate

# Windows (PowerShell)
python -m venv .venv
.venv\Scripts\Activate.ps1
```

```bash
pip install -r requirements.txt
```

> **First run:** Transformers downloads the Parakeet-TDT-v3 model (~1.2 GB) and caches it in `~/.cache/huggingface/`. This happens only once.

### 3. Build the Rust binary

```bash
# macOS / Linux / Windows
cargo build --release
```

---

## Run

```bash
cargo run --release
```

**Expected output:**

```
🎤 ParakeetSTT POC — 5s audio recording
📢 Device: MacBook Pro Microphone   ← your mic name on Windows
📐 48000 Hz | 1 channels | F32
Recording...
🔄 Resampling: 48000 Hz → 16000 Hz
✅ Saved: temp_recording.wav (80000 samples)
🔄 Running Parakeet-TDT inference...
Loading nvidia/parakeet-tdt-0.6b-v3...
Warming up...
Ready.
Transcribing 5.0s of audio...

📝 Transcription:
bonjour comment allez vous aujourd'hui
```

---

## Troubleshooting

**`cpal` build fails on Windows?**
Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the "Desktop development with C++" workload. cpal needs a C compiler.

**No input device found?**
Make sure your microphone is set as the default input device in Windows Sound settings.

**Model download slow?**
The model is ~1.2 GB. First run may take 5-10 minutes depending on your connection. HuggingFace CLI: `huggingface-cli download nvidia/parakeet-tdt-0.6b-v3`.

**Python `ModuleNotFoundError`?**
Ensure the `.venv` is activated before running `cargo run --release`. Or run the Python script directly:

```bash
source .venv/bin/activate  # macOS/Linux
python transcribe.py temp_recording.wav
```

---

## Architecture

```
┌─────────────────┐     WAV file     ┌──────────────────────┐
│  Rust binary    │ ───────────────▶ │  transcribe.py       │
│  (cpal audio)   │                 │  Parakeet-TDT-v3     │
│  5s recording   │                 │  (Transformers/PyTorch│
└─────────────────┘                 └──────────────────────┘
```

Rust handles: audio capture, resampling, WAV encoding  
Python handles: model loading, inference, CTC decoding

---

## Next steps (after POC validated)

1. Replace subprocess call with **persistent HTTP client** to `parakeet_server`
2. Add **hotkey detection** (push-to-talk)
3. Add **text injection** (clipboard + keyboard)
4. Wrap in **Tauri** for a real menu bar app
5. Build **parakeet_server** (FastAPI) for persistent background transcription

See the [PRD](https://github.com/dodmcdund-cc/parakeet-stt/tree/main/PRDs) for the full specification.
