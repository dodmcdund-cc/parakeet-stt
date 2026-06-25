use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use tauri::State;
use tracing;

use crate::app_state::{AppState as CoreState, State as CoreStateEnum};
use crate::audio::AudioCapture;
use crate::text_injector::TextInjection;

// ---------------------------------------------------------------------------
// Core state snapshot (existing)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct AppStateSnapshot {
    pub state: String,
    pub health_ok: bool,
    pub last_transcription: Option<String>,
    pub last_error: Option<String>,
}

#[tauri::command]
pub async fn get_app_state(
    state: State<'_, Arc<Mutex<CoreState>>>,
) -> Result<AppStateSnapshot, String> {
    let s = state.lock().await;
    Ok(AppStateSnapshot {
        state: format!("{:?}", s.state()),
        health_ok: s.is_health_ok(),
        last_transcription: s.last_transcription().map(|t| t.to_string()),
        last_error: s.last_error().map(|e| e.to_string()),
    })
}

// ---------------------------------------------------------------------------
// Current health check (existing)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn check_health(
    state: State<'_, Arc<Mutex<CoreState>>>,
) -> Result<bool, String> {
    let url = {
        let s = state.lock().await;
        s.server_url().to_string()
    };
    match crate::rpc_client::check_health(&url).await {
        Ok(ok) => {
            let mut s = state.lock().await;
            s.set_server_health(ok);
            Ok(ok)
        }
        Err(e) => {
            tracing::error!("Health check command failed: {}", e);
            let mut s = state.lock().await;
            s.set_server_health(false);
            Ok(false)
        }
    }
}

// ---------------------------------------------------------------------------
// Tauri IPC types for frontend
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum AppStatus {
    #[default]
    Idle,
    Recording,
    Processing,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub hotkey_keycode: u32,
    pub activation_mode: String,
    pub max_recording_duration_secs: f64,
    pub silence_threshold: f32,
    pub silence_duration_secs: f64,
    pub audio_feedback: bool,
    pub server_url: String,
    pub transcription_language: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            hotkey_keycode: 61,
            activation_mode: "pushToTalk".into(),
            max_recording_duration_secs: 60.0,
            silence_threshold: 0.02,
            silence_duration_secs: 2.0,
            audio_feedback: true,
            server_url: "http://127.0.0.1:8973".into(),
            transcription_language: "auto".into(),
        }
    }
}

pub struct ManagedState {
    pub status: std::sync::Mutex<AppStatus>,
    pub settings: std::sync::Mutex<AppSettings>,
}

impl ManagedState {
    pub fn new() -> Self {
        ManagedState {
            status: std::sync::Mutex::new(AppStatus::Idle),
            settings: std::sync::Mutex::new(AppSettings::default()),
        }
    }
}

// ---------------------------------------------------------------------------
// Tauri IPC commands (spec section 3)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn start_recording(
    managed: State<'_, ManagedState>,
    audio: State<'_, Arc<Mutex<dyn AudioCapture>>>,
) -> Result<(), String> {
    tracing::info!("Starting recording via IPC");

    let feedback = managed.settings.lock().unwrap().audio_feedback;
    if feedback {
        crate::sound::play_start_beep();
    }

    *managed.status.lock().unwrap() = AppStatus::Recording;
    audio.lock().await.start_recording()?;
    Ok(())
}

#[tauri::command]
pub async fn stop_recording(
    managed: State<'_, ManagedState>,
    audio: State<'_, Arc<Mutex<dyn AudioCapture>>>,
    injector: State<'_, Arc<Mutex<dyn TextInjection>>>,
) -> Result<(), String> {
    tracing::info!("Stopping recording via IPC");

    let samples = audio.lock().await.stop_recording()?;

    if samples.is_empty() {
        *managed.status.lock().unwrap() = AppStatus::Idle;
        return Ok(());
    }

    *managed.status.lock().unwrap() = AppStatus::Processing;

    let (server_url, language, feedback) = {
        let s = managed.settings.lock().unwrap();
        (
            s.server_url.clone(),
            s.transcription_language.clone(),
            s.audio_feedback,
        )
    };

    if feedback {
        crate::sound::play_stop_beep();
    }

    let result = crate::rpc_client::transcribe(&server_url, &samples, &language).await;

    match result {
        Ok(text) => {
            tracing::info!("Transcription received ({} chars)", text.len());
            injector.lock().await.inject(&text)?;
            *managed.status.lock().unwrap() = AppStatus::Idle;
        }
        Err(e) => {
            tracing::error!("Transcription error: {}", e);
            *managed.status.lock().unwrap() = AppStatus::Error(e.clone());
            return Err(e);
        }
    }

    Ok(())
}

#[tauri::command]
pub fn get_status(managed: State<'_, ManagedState>) -> AppStatus {
    managed.status.lock().unwrap().clone()
}

#[tauri::command]
pub fn get_settings(managed: State<'_, ManagedState>) -> AppSettings {
    managed.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_settings(
    managed: State<'_, ManagedState>,
    settings: AppSettings,
) -> Result<(), String> {
    tracing::info!("Saving settings");
    *managed.settings.lock().unwrap() = settings;
    Ok(())
}

#[tauri::command]
pub async fn inject_text(
    injector: State<'_, Arc<Mutex<dyn TextInjection>>>,
    text: String,
) -> Result<(), String> {
    tracing::info!("Injecting text via IPC ({} chars)", text.len());
    injector.lock().await.inject(&text)
}

#[tauri::command]
pub async fn check_server_health(server_url: String) -> Result<bool, String> {
    crate::rpc_client::check_health(&server_url).await
}
