use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static IS_RECORDING: AtomicBool = AtomicBool::new(false);
static RECORDING: Mutex<Vec<f32>> = Mutex::new(Vec::new());

struct StreamHandle(Option<cpal::Stream>);
unsafe impl Send for StreamHandle {}
unsafe impl Sync for StreamHandle {}

static STREAM: Mutex<StreamHandle> = Mutex::new(StreamHandle(None));

const TARGET_SAMPLE_RATE: u32 = 16000;
const SILENCE_THRESHOLD: f32 = 0.02;
const MAX_DURATION_SECS: f64 = 60.0;
const MAX_SAMPLES: usize = (TARGET_SAMPLE_RATE as f64 * MAX_DURATION_SECS) as usize;

pub fn start_recording() {
    if IS_RECORDING.swap(true, Ordering::SeqCst) {
        return;
    }

    RECORDING.lock().unwrap().clear();

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("No input device available");

    let config = device
        .default_input_config()
        .expect("Cannot get default input config");

    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;
    let is_native = sample_rate == TARGET_SAMPLE_RATE && channels == 1;

    let err_fn = |err| eprintln!("Audio stream error: {}", err);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if !IS_RECORDING.load(Ordering::Relaxed) {
                        return;
                    }

                    let mut rec = match RECORDING.lock() {
                        Ok(g) => g,
                        Err(_) => return,
                    };

                    if rec.len() >= MAX_SAMPLES {
                        return;
                    }

                    if is_native {
                        let remaining = MAX_SAMPLES.saturating_sub(rec.len());
                        rec.extend_from_slice(&data[..data.len().min(remaining)]);
                    } else {
                        let mono: Vec<f32> = data.iter().step_by(channels).copied().collect();
                        let decimation = (sample_rate / TARGET_SAMPLE_RATE).max(1) as usize;
                        let downsampled: Vec<f32> =
                            mono.iter().step_by(decimation).copied().collect();
                        let remaining = MAX_SAMPLES.saturating_sub(rec.len());
                        rec.extend_from_slice(&downsampled[..downsampled.len().min(remaining)]);
                    }
                },
                err_fn,
                None,
            )
            .expect("Failed to build input stream"),
        fmt => panic!("Unsupported sample format: {:?}", fmt),
    };

    stream.play().expect("Failed to start audio stream");
    STREAM.lock().unwrap().0 = Some(stream);
}

pub fn stop_recording() -> Vec<f32> {
    IS_RECORDING.store(false, Ordering::SeqCst);

    STREAM.lock().unwrap().0 = None;

    let samples = RECORDING.lock().unwrap().clone();
    RECORDING.lock().unwrap().clear();
    samples
}

pub fn is_recording() -> bool {
    IS_RECORDING.load(Ordering::Relaxed)
}

pub fn silence_threshold() -> f32 {
    SILENCE_THRESHOLD
}

pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|&s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

// Backward-compat trait + stub for app_state integration
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
