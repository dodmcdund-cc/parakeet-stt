import logging
from contextlib import asynccontextmanager
from fastapi import FastAPI, UploadFile, File, Form, HTTPException
from fastapi.responses import JSONResponse
import uvicorn

try:
    from transcription import ParakeetTranscriber
except ImportError as e:
    raise ImportError(
        "The 'transcription' module is required but not yet available. "
        "This module will be provided by AP-56. "
        "See AGENTS.md or the project board for more information."
    ) from e

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("parakeet_server")

transcriber: ParakeetTranscriber | None = None


@asynccontextmanager
async def lifespan(app: FastAPI):
    global transcriber
    logger.info("Loading Parakeet-TDT-0.6B-v3 model...")
    transcriber = ParakeetTranscriber()
    await transcriber.load()
    logger.info("Model loaded successfully.")
    yield
    logger.info("Shutting down, unloading model...")
    transcriber = None


app = FastAPI(
    title="ParakeetServer",
    description="HTTP server for NVIDIA Parakeet-TDT multilingual speech-to-text",
    version="1.0.0",
    lifespan=lifespan,
)


@app.get("/")
async def root():
    return {
        "name": "ParakeetServer",
        "version": "1.0.0",
        "model": "nvidia/parakeet-tdt-0.6b-v3",
    }


@app.get("/health")
async def health():
    if transcriber is None:
        return JSONResponse(
            {"status": "error", "model_loaded": False, "model_name": None},
            status_code=503,
        )
    return {
        "status": "ok",
        "model_loaded": transcriber.is_loaded(),
        "model_name": "nvidia/parakeet-tdt-0.6b-v3",
        "device": "cpu",
        "torch_version": transcriber.torch_version if hasattr(transcriber, 'torch_version') else "unknown",
    }


@app.get("/model/status")
async def model_status():
    if transcriber is None:
        return {"loaded": False, "loading": False, "error": "Server not initialized", "model_name": None}
    return transcriber.get_status()


@app.post("/model/load")
async def model_load():
    if transcriber is None:
        raise HTTPException(status_code=503, detail="Server not initialized")
    await transcriber.load()
    return {"status": "loaded", "model_name": "nvidia/parakeet-tdt-0.6b-v3"}


@app.post("/transcribe")
async def transcribe(
    audio: UploadFile = File(...),
    language: str = Form("auto"),
):
    if transcriber is None or not transcriber.is_loaded():
        raise HTTPException(status_code=503, detail="Model not yet loaded")

    if not audio.filename.lower().endswith(".wav"):
        raise HTTPException(status_code=400, detail="Only WAV files are supported")

    audio_bytes = await audio.read()

    try:
        result = await transcriber.transcribe(audio_bytes, language=language)
        return result
    except Exception as e:
        logger.error(f"Transcription error: {e}")
        raise HTTPException(status_code=500, detail=f"Transcription failed: {str(e)}")


if __name__ == "__main__":
    uvicorn.run(
        "server:app",
        host="127.0.0.1",
        port=8973,
        log_level="info",
        access_log=False,
    )
