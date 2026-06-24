use reqwest::multipart;

pub async fn check_health(server_url: &str) -> Result<bool, String> {
    let url = format!("{}/api/health", server_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    match client.get(&url).send().await {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(e) => Err(format!("Health check failed: {}", e)),
    }
}

pub async fn transcribe(
    server_url: &str,
    samples: &[f32],
    language: &str,
) -> Result<String, String> {
    let url = format!("{}/api/transcribe", server_url.trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let wav_data = encode_wav(samples);
    let file_part = multipart::Part::bytes(wav_data)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("Failed to create file part: {}", e))?;

    let language_part = multipart::Part::text(language.to_string());

    let form = multipart::Form::new()
        .part("audio", file_part)
        .part("language", language_part);

    let resp = client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Transcribe request failed: {}", e))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    if !status.is_success() {
        return Err(format!(
            "Server returned {}: {}",
            status.as_u16(),
            body
        ));
    }

    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Failed to parse response JSON: {}", e))?;

    parsed["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Response missing 'text' field: {}", body))
}

fn encode_wav(samples: &[f32]) -> Vec<u8> {
    let sample_rate = 16000u32;
    let channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let bytes_per_sample = bits_per_sample / 8;
    let block_align = channels * bytes_per_sample;
    let byte_rate = sample_rate * block_align as u32;
    let data_size = samples.len() as u32 * bytes_per_sample as u32;
    let file_size = 36 + data_size;

    let mut wav = Vec::with_capacity(44 + data_size as usize);

    // RIFF header
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    // fmt chunk
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());

    // data chunk
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());

    // PCM 16-bit samples
    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let pcm = (clamped * i16::MAX as f32) as i16;
        wav.extend_from_slice(&pcm.to_le_bytes());
    }

    wav
}
