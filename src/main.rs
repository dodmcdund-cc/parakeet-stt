use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

const SAMPLE_RATE: u32 = 16000;
const RECORD_SECONDS: u64 = 5;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 ParakeetSTT POC — {}s audio recording", RECORD_SECONDS);
    println!("Recording...");

    let samples = record_audio()?;

    let wav_path = "temp_recording.wav";
    write_wav(wav_path, &samples)?;
    println!("✅ Saved: {} ({} samples)", wav_path, samples.len());

    println!("🔄 Running Parakeet-TDT inference...");
    let result = run_transcription(wav_path)?;

    println!("\n📝 Transcription:\n{}", result);

    std::fs::remove_file(wav_path)?;

    Ok(())
}

// ─── Audio Recording ───────────────────────────────────────────────────────────

fn record_audio() -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No input device found")?;

    println!("📢 Device: {}", device.name()?);

    let config = device.default_input_config()?;
    println!(
        "📐 {} Hz | {} channels | {:?}",
        config.sample_rate().0,
        config.channels(),
        config.sample_format()
    );

    let sample_rate = config.sample_rate().0;
    let channels = config.channels();

    let (tx, rx) = std::sync::mpsc::channel::<f32>();

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            let tx = tx.clone();
            device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    for &sample in data {
                        let _ = tx.send(sample);
                    }
                },
                |err| eprintln!("Audio error: {}", err),
                None,
            )?
        }
        _ => return Err("Unsupported sample format".into()),
    };

    stream.play()?;

    let mut recorded = Vec::new();
    let start = std::time::Instant::now();

    while start.elapsed().as_secs() < RECORD_SECONDS {
        while let Ok(sample) = rx.try_recv() {
            recorded.push(sample);
        }
        thread::sleep(Duration::from_millis(50));
    }

    drop(tx);
    drop(stream);

    // Downsample to 16kHz if needed
    let samples = if sample_rate != SAMPLE_RATE {
        println!("🔄 Resampling: {} Hz → {} Hz", sample_rate, SAMPLE_RATE);
        let ratio = sample_rate as f64 / SAMPLE_RATE as f64;
        recorded
            .iter()
            .enumerate()
            .filter(|(i, _)| (*i as f64 % ratio) < 1.0)
            .map(|(_, &s)| s)
            .collect()
    } else {
        recorded
    };

    // Mix stereo to mono
    let samples = if channels > 1 {
        println!("🔄 Mixing {} channels → mono", channels);
        let chunk_size = channels as usize;
        samples
            .chunks(chunk_size)
            .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
            .collect()
    } else {
        samples
    };

    Ok(samples)
}

// ─── WAV Writer ────────────────────────────────────────────────────────────────

fn write_wav(path: &str, samples: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
    let sample_rate = SAMPLE_RATE;
    let num_channels = 1u16;
    let bits_per_sample = 32u16;
    let byte_rate = sample_rate * num_channels as u32 * bits_per_sample as u32 / 8;
    let block_align = num_channels * bits_per_sample / 8;
    let data_size = samples.len() as u32 * 4;
    let file_size = 36 + data_size;

    let mut file = File::create(path)?;

    // RIFF header
    file.write_all(b"RIFF")?;
    file.write_all(&file_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&3u16.to_le_bytes())?; // IEEE float
    file.write_all(&num_channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&bits_per_sample.to_le_bytes())?;

    // data chunk
    file.write_all(b"data")?;
    file.write_all(&data_size.to_le_bytes())?;
    for &sample in samples {
        file.write_all(&sample.to_le_bytes())?;
    }

    Ok(())
}

// ─── Python subprocess ─────────────────────────────────────────────────────────

fn run_transcription(wav_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("python3")
        .args(["transcribe.py", wav_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Python script failed:\n{}", stderr).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.into_owned())
}
