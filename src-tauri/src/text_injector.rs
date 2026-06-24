use tracing;

pub trait TextInjection: Send {
    fn inject(&mut self, text: &str) -> Result<(), String>;
}

pub struct StubTextInjector;

impl StubTextInjector {
    pub fn new() -> Self {
        StubTextInjector
    }
}

impl TextInjection for StubTextInjector {
    fn inject(&mut self, text: &str) -> Result<(), String> {
        tracing::info!("StubTextInjector: would inject text ({:?})", text);
        Ok(())
    }
}
