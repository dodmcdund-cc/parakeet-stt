# ParakeetServer API Reference

**Server:** `parakeet_server` — FastAPI application on `http://127.0.0.1:8973`

---

## `GET /`

Root endpoint returning server metadata.

**Response `200`:**

```json
{
  "name": "ParakeetServer",
  "version": "1.0.0",
  "model": "nvidia/parakeet-tdt-0.6b-v3"
}
```

---

## `GET /health`

Health check endpoint. Returns server status and model loading state.

**Response `200` (healthy):**

```json
{
  "status": "ok",
  "model_loaded": true,
  "model_name": "nvidia/parakeet-tdt-0.6b-v3",
  "device": "cpu",
  "torch_version": "2.4.0"
}
```

**Response `503` (model not loaded):**

```json
{
  "status": "error",
  "model_loaded": false,
  "model_name": null
}
```

---

## `GET /model/status`

Check model loading status with detailed state information.

**Response `200`:**

```json
{
  "loaded": true,
  "loading": false,
  "error": null,
  "model_name": "nvidia/parakeet-tdt-0.6b-v3"
}
```

**Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `loaded` | `bool` | Model is fully loaded and ready |
| `loading` | `bool` | Model is currently being loaded |
| `error` | `string \| null` | Error message if loading failed |
| `model_name` | `string \| null` | Name of the loaded model |

---

## `POST /model/load`

Explicitly trigger model loading. Useful for re-loading after a failure or pre-warming on server startup.

**Request:**

```json
{
  "model_name": "nvidia/parakeet-tdt-0.6b-v3"
}
```

**Response `200`:**

```json
{
  "status": "loaded",
  "model_name": "nvidia/parakeet-tdt-0.6b-v3"
}
```

**Error `503`:**

```json
{
  "detail": "Server not initialized"
}
```

---

## `POST /transcribe`

Transcribe a WAV audio file using the Parakeet-TDT model.

### Request

**Content-Type:** `multipart/form-data`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `audio` | `file` | Yes | WAV file (16kHz, mono, Float32 PCM) |
| `language` | `string` | No | ISO 639-1 code (e.g. `"fr"`, `"de"`) or `"auto"` (default) |

### Response `200`

```json
{
  "text": "Bonjour, comment allez-vous aujourd'hui ?",
  "language": "fr",
  "segments": [
    {
      "start": 0.0,
      "end": 1.5,
      "text": "Bonjour,"
    },
    {
      "start": 1.5,
      "end": 3.2,
      "text": "comment allez-vous aujourd'hui ?"
    }
  ],
  "duration_seconds": 3.2,
  "inference_time_ms": 2150.0
}
```

**Response fields:**

| Field | Type | Description |
|-------|------|-------------|
| `text` | `string` | Transcribed text |
| `language` | `string` | Detected or requested language code |
| `segments` | `array` | Timestamped text segments |
| `duration_seconds` | `number` | Duration of input audio |
| `inference_time_ms` | `number` | Model inference time in milliseconds |

### Error Responses

**`400` — Invalid request:**

```json
{
  "detail": "Only WAV files are supported"
}
```

```json
{
  "detail": "Failed to decode WAV: <reason>"
}
```

**`503` — Model not ready:**

```json
{
  "detail": "Model not yet loaded"
}
```

**`500` — Transcription failure:**

```json
{
  "detail": "Transcription failed: <reason>"
}
```

---

## Supported Languages

| Code | Language |
|------|----------|
| `bg` | Bulgarian |
| `hr` | Croatian |
| `cs` | Czech |
| `da` | Danish |
| `nl` | Dutch |
| `en` | English |
| `et` | Estonian |
| `fi` | Finnish |
| `fr` | French |
| `de` | German |
| `el` | Greek |
| `hu` | Hungarian |
| `it` | Italian |
| `lv` | Latvian |
| `lt` | Lithuanian |
| `mt` | Maltese |
| `pl` | Polish |
| `pt` | Portuguese |
| `ro` | Romanian |
| `sk` | Slovak |
| `sl` | Slovenian |
| `es` | Spanish |
| `sv` | Swedish |
| `ru` | Russian |
| `uk` | Ukrainian |

---

## Audio Format Requirements

| Property | Requirement |
|----------|-------------|
| Format | WAV (RIFF header, PCM) |
| Sample rate | 16,000 Hz |
| Channels | 1 (mono) |
| Bit depth | 32-bit float |
| Byte order | Little-endian |
| Min duration | 0.1 seconds |

---

## HTTP Status Code Summary

| Code | Meaning |
|------|---------|
| `200` | Success |
| `400` | Invalid request (bad file format, wrong sample rate, audio too short) |
| `500` | Internal server error (transcription failure) |
| `503` | Service unavailable (model not loaded or server not initialized) |
