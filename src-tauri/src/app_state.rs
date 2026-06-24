use std::sync::Arc;
use tokio::sync::Mutex;
use tracing;

use crate::audio::AudioCapture;
use crate::text_injector::TextInjection;

// ---------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Recording,
    Processing,
    TextInjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    ServerUnreachable,
    TranscriptionError(String),
    EmptyResult,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::ServerUnreachable => write!(f, "Server unreachable"),
            AppError::TranscriptionError(msg) => write!(f, "Transcription error: {}", msg),
            AppError::EmptyResult => write!(f, "Empty transcription result"),
        }
    }
}

impl std::error::Error for AppError {}

// ---------------------------------------------------------------------------
// AppState – thread-safe state holder
// ---------------------------------------------------------------------------

pub struct AppState {
    state: State,
    server_url: String,
    language: String,
    samples: Vec<f32>,
    last_transcription: Option<String>,
    last_error: Option<AppError>,
    health_ok: bool,
}

impl AppState {
    pub fn new(server_url: &str, language: &str) -> Self {
        AppState {
            state: State::Idle,
            server_url: server_url.to_string(),
            language: language.to_string(),
            samples: Vec::new(),
            last_transcription: None,
            last_error: None,
            health_ok: false,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn is_health_ok(&self) -> bool {
        self.health_ok
    }

    pub fn last_transcription(&self) -> Option<&str> {
        self.last_transcription.as_deref()
    }

    pub fn last_error(&self) -> Option<&AppError> {
        self.last_error.as_ref()
    }

    pub fn server_url(&self) -> &str {
        &self.server_url
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    pub fn set_server_health(&mut self, ok: bool) {
        self.health_ok = ok;
        if !ok {
            self.last_error = Some(AppError::ServerUnreachable);
        }
    }

    fn transition_to_recording(&mut self) -> Result<(), AppError> {
        if self.state != State::Idle {
            return Err(AppError::TranscriptionError(format!(
                "Cannot start recording in state {:?}",
                self.state
            )));
        }
        self.state = State::Recording;
        self.samples.clear();
        self.last_error = None;
        tracing::info!("State transition: Idle -> Recording");
        Ok(())
    }

    fn transition_to_processing(&mut self, samples: Vec<f32>) -> Result<(), AppError> {
        if self.state != State::Recording {
            return Err(AppError::TranscriptionError(format!(
                "Cannot process in state {:?}",
                self.state
            )));
        }
        self.samples = samples;
        self.state = State::Processing;
        tracing::info!("State transition: Recording -> Processing");
        Ok(())
    }

    fn transition_to_text_injection(&mut self, text: String) -> Result<(), AppError> {
        if self.state != State::Processing {
            return Err(AppError::TranscriptionError(format!(
                "Cannot inject in state {:?}",
                self.state
            )));
        }
        if text.is_empty() {
            self.state = State::Idle;
            self.last_error = Some(AppError::EmptyResult);
            tracing::warn!("Empty transcription: silent discard, returning to Idle");
            return Err(AppError::EmptyResult);
        }
        self.last_transcription = Some(text);
        self.state = State::TextInjection;
        tracing::info!("State transition: Processing -> TextInjection");
        Ok(())
    }

    fn transition_to_idle(&mut self) {
        self.state = State::Idle;
        self.samples.clear();
        tracing::info!("State transition: -> Idle");
    }
}

// ---------------------------------------------------------------------------
// AppController – orchestrates the full data flow
// ---------------------------------------------------------------------------

type SharedState = Arc<Mutex<AppState>>;

pub struct AppController {
    state: SharedState,
}

impl AppController {
    pub fn new(state: SharedState) -> Self {
        AppController { state }
    }

    pub fn state(&self) -> &SharedState {
        &self.state
    }

    /// Health check against the transcription server.
    /// Sets `health_ok` flag on AppState accordingly.
    pub async fn check_server_health(&self) -> bool {
        let url = {
            let s = self.state.lock().await;
            s.server_url().to_string()
        };
        match crate::rpc_client::check_health(&url).await {
            Ok(ok) => {
                let mut s = self.state.lock().await;
                s.set_server_health(ok);
                ok
            }
            Err(e) => {
                tracing::warn!("Server health check failed: {}", e);
                let mut s = self.state.lock().await;
                s.set_server_health(false);
                false
            }
        }
    }

    /// Handle a hotkey press event.
    /// Idle → Recording: starts audio capture.
    /// Other states: ignored.
    pub async fn on_hotkey_press(&self, audio: Arc<Mutex<dyn AudioCapture>>) {
        let mut s = self.state.lock().await;
        if s.state() != State::Idle {
            tracing::warn!(
                "on_hotkey_press ignored: state is {:?} (expected Idle)",
                s.state()
            );
            return;
        }
        if let Err(e) = s.transition_to_recording() {
            tracing::error!("State transition to Recording failed: {}", e);
            return;
        }
        drop(s);

        if let Err(e) = audio.lock().await.start_recording() {
            tracing::error!("Failed to start audio capture: {}", e);
            let mut s = self.state.lock().await;
            s.transition_to_idle();
        }
    }

    /// Handle a hotkey release event.
    /// Recording → Processing → RPC transcribe → TextInjection → Idle.
    pub async fn on_hotkey_release(
        &self,
        audio: Arc<Mutex<dyn AudioCapture>>,
        injector: Arc<Mutex<dyn TextInjection>>,
    ) {
        let samples = {
            let mut a = audio.lock().await;
            match a.stop_recording() {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to stop audio capture: {}", e);
                    let mut s = self.state.lock().await;
                    s.transition_to_idle();
                    return;
                }
            }
        };

        let (server_url, language) = {
            let mut s = self.state.lock().await;
            if s.state() != State::Recording {
                tracing::warn!(
                    "on_hotkey_release called but state is {:?} (expected Recording)",
                    s.state()
                );
                return;
            }
            if let Err(e) = s.transition_to_processing(samples.clone()) {
                tracing::error!("State transition to Processing failed: {}", e);
                s.transition_to_idle();
                return;
            }
            (s.server_url().to_string(), s.language().to_string())
        };

        let result = crate::rpc_client::transcribe(&server_url, &samples, &language).await;

        let text = match result {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Transcription failed: {}", e);
                let mut s = self.state.lock().await;
                s.transition_to_idle();
                return;
            }
        };

        let skip_inject = {
            let mut s = self.state.lock().await;
            if s.state() != State::Processing {
                return;
            }
            match s.transition_to_text_injection(text.clone()) {
                Ok(()) => false,
                Err(AppError::EmptyResult) => true,
                Err(e) => {
                    tracing::error!("State transition to TextInjection failed: {}", e);
                    s.transition_to_idle();
                    true
                }
            }
        };

        if skip_inject {
            return;
        }

        if let Err(e) = injector.lock().await.inject(&text) {
            tracing::error!("Text injection failed: {}", e);
        }

        let mut s = self.state.lock().await;
        s.transition_to_idle();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> SharedState {
        Arc::new(Mutex::new(AppState::new("http://127.0.0.1:8973", "auto")))
    }

    #[tokio::test]
    async fn test_initial_state_is_idle() {
        let state = make_state();
        let s = state.lock().await;
        assert_eq!(s.state(), State::Idle);
        assert!(!s.is_health_ok());
        assert!(s.last_transcription().is_none());
        assert!(s.last_error().is_none());
    }

    #[tokio::test]
    async fn test_idle_to_recording() {
        let state = make_state();
        {
            let mut s = state.lock().await;
            assert!(s.transition_to_recording().is_ok());
        }
        {
            let s = state.lock().await;
            assert_eq!(s.state(), State::Recording);
        }
    }

    #[tokio::test]
    async fn test_recording_to_processing() {
        let state = make_state();
        {
            let mut s = state.lock().await;
            s.transition_to_recording().unwrap();
            s.transition_to_processing(vec![0.0; 16000]).unwrap();
        }
        {
            let s = state.lock().await;
            assert_eq!(s.state(), State::Processing);
        }
    }

    #[tokio::test]
    async fn test_processing_to_text_injection() {
        let state = make_state();
        {
            let mut s = state.lock().await;
            s.transition_to_recording().unwrap();
            s.transition_to_processing(vec![0.0; 16000]).unwrap();
            s.transition_to_text_injection("hello world".into()).unwrap();
        }
        {
            let s = state.lock().await;
            assert_eq!(s.state(), State::TextInjection);
            assert_eq!(s.last_transcription(), Some("hello world"));
        }
    }

    #[tokio::test]
    async fn test_full_cycle() {
        let state = make_state();
        {
            let mut s = state.lock().await;
            s.transition_to_recording().unwrap();
            s.transition_to_processing(vec![0.0; 16000]).unwrap();
            s.transition_to_text_injection("test".into()).unwrap();
            s.transition_to_idle();
        }
        {
            let s = state.lock().await;
            assert_eq!(s.state(), State::Idle);
            assert_eq!(s.last_transcription(), Some("test"));
        }
    }

    #[tokio::test]
    async fn test_empty_text_discarded() {
        let state = make_state();
        {
            let mut s = state.lock().await;
            s.transition_to_recording().unwrap();
            s.transition_to_processing(vec![0.0; 16000]).unwrap();
            let result = s.transition_to_text_injection(String::new());
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), AppError::EmptyResult);
        }
        {
            let s = state.lock().await;
            assert_eq!(s.state(), State::Idle);
            assert!(s.last_transcription().is_none());
        }
    }

    #[tokio::test]
    async fn test_invalid_transition_from_idle_to_processing() {
        let state = make_state();
        let mut s = state.lock().await;
        let result = s.transition_to_processing(vec![0.0; 16000]);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_server_health() {
        let state = make_state();
        assert!(!state.lock().await.is_health_ok());
        state.lock().await.set_server_health(true);
        assert!(state.lock().await.is_health_ok());
        state.lock().await.set_server_health(false);
        assert!(!state.lock().await.is_health_ok());
        assert_eq!(
            state.lock().await.last_error(),
            Some(&AppError::ServerUnreachable)
        );
    }

    #[tokio::test]
    async fn test_double_recording_is_rejected() {
        let state = make_state();
        let mut s = state.lock().await;
        s.transition_to_recording().unwrap();
        let result = s.transition_to_recording();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_controller_hotkey_press_release_with_stubs() {
        let state = make_state();
        let controller = AppController::new(state.clone());
        let audio = Arc::new(Mutex::new(crate::audio::StubAudioCapture::new()))
            as Arc<Mutex<dyn AudioCapture>>;
        let injector = Arc::new(Mutex::new(crate::text_injector::StubTextInjector))
            as Arc<Mutex<dyn TextInjection>>;

        controller.on_hotkey_press(audio.clone()).await;
        {
            let s = state.lock().await;
            assert_eq!(s.state(), State::Recording);
        }

        controller
            .on_hotkey_release(audio.clone(), injector.clone())
            .await;
        {
            let s = state.lock().await;
            assert_eq!(s.state(), State::Idle);
        }
    }

    #[tokio::test]
    async fn test_press_in_wrong_state_is_ignored() {
        let state = make_state();
        let controller = AppController::new(state.clone());
        let audio = Arc::new(Mutex::new(crate::audio::StubAudioCapture::new()))
            as Arc<Mutex<dyn AudioCapture>>;
        let injector = Arc::new(Mutex::new(crate::text_injector::StubTextInjector))
            as Arc<Mutex<dyn TextInjection>>;

        controller.on_hotkey_press(audio.clone()).await;
        controller.on_hotkey_press(audio.clone()).await;
        {
            let s = state.lock().await;
            assert_eq!(s.state(), State::Recording);
        }

        controller.on_hotkey_release(audio, injector).await;
        {
            let s = state.lock().await;
            assert_eq!(s.state(), State::Idle);
        }
    }
}
