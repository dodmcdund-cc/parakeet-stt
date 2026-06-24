use std::sync::Arc;
use tokio::sync::Mutex;
use serde::Serialize;
use tauri::State;
use tracing;

use crate::app_state::{AppState, State};

#[derive(Debug, Clone, Serialize)]
pub struct AppStateSnapshot {
    pub state: String,
    pub health_ok: bool,
    pub last_transcription: Option<String>,
    pub last_error: Option<String>,
}

#[tauri::command]
pub async fn get_app_state(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<AppStateSnapshot, String> {
    let s = state.lock().await;
    Ok(AppStateSnapshot {
        state: format!("{:?}", s.state()),
        health_ok: s.is_health_ok(),
        last_transcription: s.last_transcription().map(|t| t.to_string()),
        last_error: s.last_error().map(|e| e.to_string()),
    })
}

#[tauri::command]
pub async fn check_health(
    state: State<'_, Arc<Mutex<AppState>>>,
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
