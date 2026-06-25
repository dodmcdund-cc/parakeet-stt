# ParakeetSTT

Multilingual speech-to-text desktop application powered by NVIDIA Parakeet-TDT.

ParakeetSTT is a cross-platform menu-bar application that provides real-time multilingual speech-to-text transcription using NVIDIA's Parakeet-TDT-0.6B-v3 model. It captures microphone audio, transcribes it via a local Python server, and injects the resulting text at the cursor position in any application.

**Key features:**
- 25+ supported European languages
- System-wide hotkey activation (push-to-talk or toggle)
- Menu bar UI (no dock icon)
- Automatic silence detection
- Cursor-position text injection (clipboard + keyboard simulation)
- Cross-platform: macOS, Windows, Linux

## Architecture

ParakeetSTT uses a two-process architecture:

- **[Tauri app](src/)** (Rust + Vue 3) — audio capture, global hotkeys, text injection, menu bar UI
- **[parakeet_server](parakeet_server/)** (Python + FastAPI) — model loading, ML inference, transcription

The processes communicate via HTTP on `127.0.0.1:8973`.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for details.

## Platform Requirements

| Component | Requirement |
|-----------|-------------|
| OS | macOS 13+, Windows 10+, Linux |
| Python | 3.11+ |
| Node.js | 18+ |
| Rust | 1.70+ (`rustup`) |
| RAM | ~2 GB free (for model loading) |
| Disk | ~3 GB (model + dependencies) |

### Linux Dependencies

Tauri requires GTK 3 and related system libraries:

```bash
sudo apt install libgtk-3-dev libpango1.0-dev libatk1.0-dev libgdk-pixbuf2.0-dev libcairo2-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev
```

## Installation

### 1. Clone the repository

```bash
git clone https://github.com/dodmcdund-cc/parakeet-stt
cd parakeet-stt
```

### 2. Set up the Python server

```bash
cd parakeet_server
python3 -m venv .venv
source .venv/bin/activate   # Windows: .venv\Scripts\activate
pip install -r requirements.txt
```

### 3. Start the transcription server

```bash
source .venv/bin/activate   # Windows: .venv\Scripts\activate
uvicorn server:app --host 127.0.0.1 --port 8973
```

The server loads the model on startup (~15–30s cold start). Keep it running in a terminal.

### 4. Install frontend dependencies

```bash
cd ../src
npm install
```

### 5. Build the Tauri application

```bash
npm run tauri build
```

The binary will be at:
- macOS/Linux: `src-tauri/target/release/parakeet-stt`
- Windows: `src-tauri/target/release/parakeet-stt.exe`

### 6. Run (development mode)

```bash
npm run tauri dev
```

## Model

ParakeetSTT uses **NVIDIA Parakeet-TDT-0.6B-v3**, a FastConformer-Transducer model supporting 25+ European languages.

| Property | Value |
|----------|-------|
| Architecture | FastConformer-Transducer (TDT) |
| Parameters | 600M |
| Size | ~1.2 GB (FP32) |
| Languages | 25 European languages |
| Input | 16kHz mono audio |
| Download | Auto-downloaded from HuggingFace on first run |
| Cache | `~/.cache/huggingface/` |

The model is automatically downloaded from HuggingFace on first server launch.

## API

See [docs/API.md](docs/API.md) for the complete parakeet_server API reference.

## Project Structure

```
parakeet-stt/
├── src/                          # Tauri app frontend + Rust backend
│   ├── src-tauri/                # Rust backend (Tauri)
│   │   ├── src/
│   │   │   ├── main.rs           # Entry point
│   │   │   ├── lib.rs            # Tauri app setup, IPC commands
│   │   │   ├── audio.rs          # Audio capture (cpal)
│   │   │   ├── hotkey.rs         # Global hotkeys
│   │   │   ├── text_injector.rs  # Text injection (clipboard + enigo)
│   │   │   ├── rpc_client.rs     # HTTP client to parakeet_server
│   │   │   ├── permissions.rs    # Permission handling
│   │   │   └── sound.rs          # Audio feedback
│   │   ├── Cargo.toml
│   │   └── tauri.conf.json
│   ├── src/                      # Vue 3 frontend
│   │   ├── App.vue
│   │   ├── views/
│   │   │   ├── MenuBarView.vue   # Menu bar popover
│   │   │   └── SettingsView.vue  # Settings window
│   │   ├── components/
│   │   │   ├── StatusIndicator.vue
│   │   │   ├── AudioLevelMeter.vue
│   │   │   └── HotkeyRecorder.vue
│   │   ├── stores/
│   │   │   └── app.ts            # State management (Pinia)
│   │   └── main.ts
│   ├── index.html
│   ├── package.json
│   └── vite.config.ts
├── parakeet_server/              # Python transcription server
│   ├── server.py                 # FastAPI application
│   ├── transcription.py          # Parakeet pipeline wrapper
│   ├── audio_utils.py            # WAV processing utilities
│   └── requirements.txt
├── docs/
│   ├── ARCHITECTURE.md
│   └── API.md
└── README.md
```

## Configuration

Settings are available in the app's Settings window (General, Audio, Transcription, About tabs):

| Setting | Default | Description |
|---------|---------|-------------|
| Hotkey | Right Option | Global hotkey for push-to-talk |
| Activation mode | pushToTalk | Push-to-talk or double-tap toggle |
| Max recording duration | 60s | Maximum recording length |
| Silence threshold | 0.02 | RMS threshold for silence detection |
| Silence duration | 2.0s | Silence duration before auto-stop |
| Server URL | `http://127.0.0.1:8973` | parakeet_server address |
| Language | auto | Transcription language (auto or explicit) |

## Licenses

| Component | License |
|-----------|---------|
| Tauri app (Rust + Vue frontend) | MIT |
| Python server (`parakeet_server/`) | Apache 2.0 |
| Model (NVIDIA Parakeet-TDT-0.6B-v3) | NVIDIA AI Foundation Models — see [HuggingFace model card](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3) |
