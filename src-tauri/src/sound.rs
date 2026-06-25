//! Audio feedback beeps using rodio.

static BEEP_START: &[u8] = include_bytes!("beep_start.wav");
static BEEP_STOP: &[u8] = include_bytes!("beep_stop.wav");

fn play_wav(wav_bytes: &[u8]) -> Result<(), String> {
    let cursor = std::io::Cursor::new(wav_bytes.to_vec());
    let (_stream, handle) = rodio::OutputStream::try_default().map_err(|e| e.to_string())?;
    handle.play_once(cursor).map_err(|e| e.to_string())?;
    Ok(())
}

/// Play start recording beep (high pitch ~880Hz).
pub fn play_start_beep() {
    let _ = play_wav(BEEP_START);
}

/// Play stop recording beep (lower pitch ~440Hz).
pub fn play_stop_beep() {
    let _ = play_wav(BEEP_STOP);
}
