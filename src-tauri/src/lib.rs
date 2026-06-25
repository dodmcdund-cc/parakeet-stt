pub mod app_state;
pub mod audio;
pub mod commands;
pub mod hotkey;
pub mod rpc_client;
pub mod sound;
pub mod text_injector;

use crate::hotkey::HotkeyManager;
use crate::commands::ManagedState;

pub use audio::{is_recording, rms, start_recording, stop_recording};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    let app_state = std::sync::Arc::new(tokio::sync::Mutex::new(app_state::AppState::new(
        "http://127.0.0.1:8973",
        "auto",
    )));
    let controller = std::sync::Arc::new(app_state::AppController::new(app_state.clone()));

    {
        let health_ctrl = controller.clone();
        tauri::async_runtime::spawn(async move {
            let ok = health_ctrl.check_server_health().await;
            if ok {
                tracing::info!("Server health check: OK");
            } else {
                tracing::warn!("Server health check: UNREACHABLE");
            }
        });
    }

    let audio = std::sync::Arc::new(tokio::sync::Mutex::new(audio::StubAudioCapture::new()));
    let injector = std::sync::Arc::new(tokio::sync::Mutex::new(text_injector::StubTextInjector));

    let mut hotkey_manager = hotkey::StubHotkeyManager::new();
    {
        let ctrl = controller.clone();
        let a = audio.clone();
        hotkey_manager.set_on_press(Box::new(move || {
            let ctrl = ctrl.clone();
            let a = a.clone();
            tauri::async_runtime::spawn(async move {
                ctrl.on_hotkey_press(a).await;
            });
        }));
    }
    {
        let ctrl = controller.clone();
        let a = audio.clone();
        let i = injector.clone();
        hotkey_manager.set_on_release(Box::new(move || {
            let ctrl = ctrl.clone();
            let a = a.clone();
            let i = i.clone();
            tauri::async_runtime::spawn(async move {
                ctrl.on_hotkey_release(a, i).await;
            });
        }));
    }

    if let Err(e) = hotkey_manager.start() {
        tracing::error!("Failed to start hotkey manager: {}", e);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--flag-here"]),
        ))
        .manage(app_state.clone())
        .manage(ManagedState::new())
        .manage(audio.clone())
        .manage(injector.clone())
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::get_app_state,
            commands::check_health,
            commands::start_recording,
            commands::stop_recording,
            commands::get_status,
            commands::get_settings,
            commands::save_settings,
            commands::check_server_health,
            commands::inject_text,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
