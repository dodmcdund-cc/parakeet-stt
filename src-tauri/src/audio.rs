use tracing;

pub trait AudioCapture: Send {
    fn start_recording(&mut self) -> Result<(), String>;
    fn stop_recording(&mut self) -> Result<Vec<f32>, String>;
    fn is_recording(&self) -> bool;
}

pub struct StubAudioCapture {
    recording: bool,
}

impl StubAudioCapture {
    pub fn new() -> Self {
        StubAudioCapture { recording: false }
    }
}

impl AudioCapture for StubAudioCapture {
    fn start_recording(&mut self) -> Result<(), String> {
        self.recording = true;
        tracing::info!("StubAudioCapture: recording started");
        Ok(())
    }

    fn stop_recording(&mut self) -> Result<Vec<f32>, String> {
        self.recording = false;
        tracing::info!("StubAudioCapture: recording stopped, returning 1s silence");
        Ok(vec![0.0; 16000])
    }

    fn is_recording(&self) -> bool {
        self.recording
    }
}
