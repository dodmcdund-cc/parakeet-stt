use clipboard::{ClipboardContext, ClipboardProvider};
use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings,
};
use std::thread;
use std::time::Duration;
use tracing;

pub trait TextInjection: Send {
    fn inject(&mut self, text: &str) -> Result<(), String>;
}

pub struct TextInjector;

impl TextInjector {
    pub fn new() -> Self {
        TextInjector
    }
}

impl TextInjection for TextInjector {
    fn inject(&mut self, text: &str) -> Result<(), String> {
        tracing::info!("TextInjector: injecting text ({:?})", text);

        let mut clip: ClipboardContext =
            ClipboardProvider::new().map_err(|e| format!("Failed to create clipboard: {}", e))?;

        let original = clip
            .get_contents()
            .map_err(|e| format!("Failed to read clipboard: {}", e))?;

        clip.set_contents(text.to_string())
            .map_err(|e| format!("Failed to set clipboard: {}", e))?;

        thread::sleep(Duration::from_millis(20));

        let mut enigo =
            Enigo::new(&Settings::default()).map_err(|e| format!("Failed to create enigo: {}", e))?;

        let modifier = if cfg!(target_os = "macos") {
            Key::Meta
        } else {
            Key::Control
        };

        enigo
            .key(modifier, Press)
            .map_err(|e| format!("Failed to press modifier: {}", e))?;
        enigo
            .key(Key::Unicode('v'), Click)
            .map_err(|e| format!("Failed to press V: {}", e))?;
        enigo
            .key(modifier, Release)
            .map_err(|e| format!("Failed to release modifier: {}", e))?;

        thread::sleep(Duration::from_millis(50));

        clip.set_contents(original)
            .map_err(|e| format!("Failed to restore clipboard: {}", e))?;

        tracing::info!("TextInjector: text injected successfully");
        Ok(())
    }
}
