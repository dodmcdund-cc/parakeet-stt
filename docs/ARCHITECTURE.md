# ParakeetSTT Architecture

## Overview

ParakeetSTT is a cross-platform menu-bar application providing real-time multilingual speech-to-text transcription. It uses a **two-process architecture**: a Rust/Tauri client app for audio capture and text injection, and a Python server for ML-based transcription via NVIDIA Parakeet-TDT-0.6B-v3.

## Two-Process Architecture

```
┌──────────────────────────────────────────────────────────┐
│                ParakeetSTT (Tauri App)                    │
│                        Rust + Vue 3                       │
│                                                           │
│  ┌─────────────────┐   ┌─────────────────┐              │
│  │   Web UI (Vue)   │   │  Rust Backend    │              │
│  │  - MenuBarView   │◄──│  - audio.rs      │              │
│  │  - SettingsView  │   │  - hotkey.rs     │              │
│  │  - Pinia store   │   │  - text_injector │              │
│  │                  │   │  - rpc_client    │              │
│  │                  │   │  - permissions   │              │
│  │                  │   │  - sound         │              │
│  └─────────────────┘   └────────┬─────────┘              │
│                                  │ Tauri IPC              │
└──────────────────────────────────┼────────────────────────┘
                                   │ HTTP (TCP 127.0.0.1:8973)
                                   ▼
┌──────────────────────────────────────────────────────────┐
│               parakeet_server (Python)                     │
│        FastAPI + Transformers + Parakeet-TDT-0.6B-v3      │
│                                                           │
│  ┌────────────────┐  ┌──────────────────┐                │
│  │  server.py      │  │ transcription.py  │                │
│  │  - FastAPI app  │──│  - Model loading  │                │
│  │  - Endpoints    │  │  - Inference      │                │
│  │  - Lifespan mgmt│  │  - Warm-up        │                │
│  └────────────────┘  └──────────────────┘                │
│                                                           │
│  Model: ~1.2 GB RAM (CPU, FP32)                           │
│  Cache: ~/.cache/huggingface/                             │
└──────────────────────────────────────────────────────────┘
```

### Why Two Processes?

- **Python ecosystem**: Transformers/PyTorch is Python-native; embedding it in Rust is complex
- **Fault isolation**: if the model crashes, the UI stays alive
- **Independent lifecycle**: server and UI can be updated/patched separately
- **Cross-platform**: Python server runs identically on macOS, Windows, Linux

## Service Responsibilities

### Tauri App (Rust + Vue 3)

| Module | File | Responsibility |
|--------|------|----------------|
| `audio.rs` | `src-tauri/src/audio.rs` | Microphone capture via `cpal`, 16kHz mono Float32 PCM, silence detection |
| `hotkey.rs` | `src-tauri/src/hotkey.rs` | Global hotkey registration via `hotkey` crate (push-to-talk / toggle) |
| `rpc_client.rs` | `src-tauri/src/rpc_client.rs` | HTTP client to parakeet_server, multipart WAV upload, health checks |
| `text_injector.rs` | `src-tauri/src/text_injector.rs` | Clipboard-based text injection via `enigo` keyboard simulation + `clipboard` crate |
| `sound.rs` | `src-tauri/src/sound.rs` | Audio feedback beeps via `rodio` |
| `permissions.rs` | `src-tauri/src/permissions.rs` | Microphone and accessibility permission handling |
| `lib.rs` | `src-tauri/src/lib.rs` | Tauri entry point, tray icon, IPC command handlers |
| `stores/app.ts` | `src/stores/app.ts` | Pinia store: app state, settings, transcription results |
| `views/MenuBarView.vue` | `src/views/MenuBarView.vue` | Menu bar popover UI |
| `views/SettingsView.vue` | `src/views/SettingsView.vue` | Settings window (General, Audio, Transcription, About) |

### parakeet_server (Python)

| Module | File | Responsibility |
|--------|------|----------------|
| `server.py` | `parakeet_server/server.py` | FastAPI application, endpoint definitions, lifespan management |
| `transcription.py` | `parakeet_server/transcription.py` | `ParakeetTranscriber` class: model loading, pipeline warm-up, inference |
| `audio_utils.py` | `parakeet_server/audio_utils.py` | WAV validation utilities |
| `requirements.txt` | `parakeet_server/requirements.txt` | Python dependencies |

## Data Flow

```
User presses hotkey
       │
       ▼
hotkey.rs: on_press() callback
       │
       ▼
audio.rs: start_recording()
  → cpal input stream
  → 16kHz mono Float32 PCM
  → samples accumulate in buffer
       │
       │ (silence detected OR max duration OR hotkey release)
       ▼
audio.rs: stop_recording()
  → returns Vec<f32> samples
       │
       ▼
rpc_client.rs: transcribe()
  → encodes samples as WAV (RIFF header + float32 PCM data)
  → POST /transcribe (multipart/form-data)
  → audio.wav + language parameter
  → timeout: 120s
       │
       ▼
parakeet_server (Python):
  1. Receive WAV bytes
  2. Validate format (16kHz, mono, float32)
  3. Decode via soundfile → np.ndarray
  4. Normalize audio to [-1, 1]
  5. Run pipeline: feature extraction → model forward → decoder
  6. Return JSON {text, language, segments, duration_seconds, inference_time_ms}
       │
       ▼
rpc_client.rs: parse TranscriptionResult
       │
       ▼
lib.rs: update AppState
  → last_transcription = text
  → status = Idle
       │
       ├──► Vue UI: MenuBarView shows last transcription
       │
       └──► text_injector.rs: inject(text)
                ├──► Save current clipboard
                ├──► Set clipboard to transcription text
                ├──► Simulate Cmd+V (macOS) / Ctrl+V (Win/Linux)
                └──► Restore clipboard
```

## State Machine

```
APP LAUNCH
    │
    ├─► Check permissions (microphone, accessibility)
    │       │
    │       ├─► Denied → show error indicator, stay in IDLE
    │
    ├─► Connect to parakeet_server (GET /health)
    │       │
    │       ├─► Unreachable → ERROR state, badge indicator
    │       ├─► Reachable → proceed
    │
    └─► Load settings from store
            │
            ▼
        IDLE
         │  ▲
         │  │  hotkey press / toggle
         │  │       │
         │  │       ▼
         │  │   RECORDING
         │  │       │
         │  │  silence detected / max duration / hotkey release
         │  │       │
         │  │       ▼
         │  │   PROCESSING
         │  │       │
         │  │  transcription success
         │  │       │
         │  │       ▼
         │  │  TEXT_INJECTION
         │  │       │
         │  └───────┘
         │
         └──► ERROR
                  │
                  ├─► server unreachable → retry on next action
                  ├─► transcription failed → show error toast
                  └─► empty result → silent discard → IDLE
```

## Audio Format Contract

| Stage | Format | Details |
|-------|--------|---------|
| Microphone input | Device-dependent | Usually 44.1kHz or 48kHz, stereo |
| After conversion | Float32 PCM | 16kHz, mono, [-1.0, 1.0] |
| Audio buffer | `Vec<f32>` | Rust vector of samples |
| HTTP upload | WAV file | RIFF header + float32 PCM data |
| Server input | `np.ndarray` | shape `(n_samples,)`, dtype `float32` |
| Server output | JSON | UTF-8 text |

## Error Handling

| Error | User-Facing Behavior |
|-------|---------------------|
| Microphone permission denied | Red badge indicator, tooltip message |
| Accessibility permission denied | Red badge indicator, tooltip message |
| Server unreachable | Warning badge, "Start parakeet_server in terminal" |
| Server not ready (503) | Error message: model still loading |
| Transcription error (500) | Error state with message |
| Empty transcription | Silent discard, no injection |
| Audio too short | Silent discard |
| Invalid WAV format | 400 error returned to client |
| Audio device disconnected | Auto-stop recording |

## Tech Stack

| Component | Technology | Role |
|-----------|------------|------|
| Desktop app | Rust + Tauri 2 | Native system integration, audio, hotkeys |
| UI | Vue 3 + TypeScript + Pinia | Menu bar UI, settings |
| Server | Python 3.11+ + FastAPI | ML inference |
| ML | PyTorch + Transformers | Model loading and inference |
| Model | NVIDIA Parakeet-TDT-0.6B-v3 | Speech-to-text (25 languages) |
| Audio | cpal (Rust), soundfile (Python) | Cross-platform audio I/O |
| Communication | HTTP (TCP localhost:8973) | Client-server RPC |
