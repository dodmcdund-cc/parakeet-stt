use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing;

pub trait HotkeyManager: Send {
    fn set_on_press(&mut self, cb: Box<dyn Fn() + Send>);
    fn set_on_release(&mut self, cb: Box<dyn Fn() + Send>);
    fn start(&mut self) -> Result<(), String>;
    fn stop(&mut self);
}

pub struct StubHotkeyManager {
    on_press: Option<Box<dyn Fn() + Send>>,
    on_release: Option<Box<dyn Fn() + Send>>,
    started: bool,
}

impl StubHotkeyManager {
    pub fn new() -> Self {
        StubHotkeyManager {
            on_press: None,
            on_release: None,
            started: false,
        }
    }

    pub fn simulate_press(&self) {
        if let Some(ref cb) = self.on_press {
            (cb)();
        }
    }

    pub fn simulate_release(&self) {
        if let Some(ref cb) = self.on_release {
            (cb)();
        }
    }
}

impl HotkeyManager for StubHotkeyManager {
    fn set_on_press(&mut self, cb: Box<dyn Fn() + Send>) {
        self.on_press = Some(cb);
    }

    fn set_on_release(&mut self, cb: Box<dyn Fn() + Send>) {
        self.on_release = Some(cb);
    }

    fn start(&mut self) -> Result<(), String> {
        if self.started {
            return Err("Hotkey manager already started".into());
        }
        self.started = true;
        tracing::info!("StubHotkeyManager started (no real system hotkey bound)");
        Ok(())
    }

    fn stop(&mut self) {
        self.started = false;
        tracing::info!("StubHotkeyManager stopped");
    }
}
