# ParakeetSTT — E2E Testing Checklist

> **Test plan for AP-53:** Cross-platform E2E verification of the full integration:
> hotkey → audio capture → RPC transcribe → text injection.
>
> **Primary target:** macOS 13+ · **Secondary:** Windows 10+
> **Status key:** ✅ Pass · ❌ Fail · ⚠️ Blocked · ➖ N/A

---

## Prerequisites

Before running any test:

1. **Build the app:**
   ```bash
   cd src-tauri && cargo build --release
   ```
2. **Start the Python server:**
   ```bash
   cd parakeet_server
   source .venv/bin/activate
   uvicorn server:app --host 127.0.0.1 --port 8973
   ```
   Wait for the model warm-up (~15-30 s) — server logs `"Warm-up complete"`.
3. **Grant permissions** (one-time):
   - **macOS:** Microphone + Accessibility (System Settings → Privacy & Security)
   - **Windows:** Microphone permission when prompted
4. **Launch the app** (`src-tauri/target/release/parakeet-stt`).
5. **Verify no error badge** on tray icon (server reachable, permissions OK).

---

## 1. Tray Icon & Menu Bar

| # | Test Case | Steps | Expected Result | macOS | Windows |
|---|-----------|-------|-----------------|-------|---------|
| 1.1 | Tray icon visible | Launch app, look for icon in menu bar / system tray | ParakeetSTT icon appears in macOS menu bar (no Dock icon) or Windows system tray | | |
| 1.2 | No Dock icon (macOS) | Check Dock after launch | No persistent icon in macOS Dock (`.app` is LSUIElement) | | |
| 1.3 | Click opens popover | Left-click / tap the tray icon | Popover window appears with status, microphone button, and last transcription | | |
| 1.4 | Click closes popover | Click tray icon again while popover is open | Popover closes | | |
| 1.5 | Click outside closes popover | Open popover, click anywhere outside it | Popover closes automatically | | |
| 1.6 | Right-click context menu | Right-click tray icon | Context menu appears with Settings / Quit options | | |

---

## 2. Hotkey — Start / Stop Recording

| # | Test Case | Steps | Expected Result | macOS | Windows |
|---|-----------|-------|-----------------|-------|---------|
| 2.1 | Push-to-talk press starts recording | Press hotkey (default: Right Option) | Status indicator switches to "Recording", red dot appears | | |
| 2.2 | Push-to-talk release stops recording | Release hotkey | Status switches to "Processing" then "Idle", transcription appears | | |
| 2.3 | Toggle mode — first press starts | Set mode to "Toggle" in settings, press hotkey | Recording starts, stays recording after release | | |
| 2.4 | Toggle mode — second press stops | Press hotkey again while recording | Recording stops, processing + transcription flow runs | | |
| 2.5 | Hotkey ignored during processing | Start recording, release, immediately press hotkey again | Press ignored — stays in Processing state | | |
| 2.6 | Double press does not double-record | Press hotkey twice quickly | Second press ignored, single recording active | | |
| 2.7 | Hotkey works in any app | Switch to another app, press hotkey | Recording still starts correctly (global hotkey) | | |

---

## 3. Audio Level Animation

| # | Test Case | Steps | Expected Result | macOS | Windows |
|---|-----------|-------|-----------------|-------|---------|
| 3.1 | Level meter visible during recording | Press hotkey, look at popover | Audio level meter animates in real-time during recording | | |
| 3.2 | Meter responds to voice | Speak into microphone while recording | Level bars fluctuate with speech volume | | |
| 3.3 | Meter at zero when silent | Stay silent while recording | Level meter shows near-zero or flat line | | |
| 3.4 | Meter stops after recording | Release hotkey | Meter freezes or disappears when recording stops | | |

---

## 4. Silence Detection

| # | Test Case | Steps | Expected Result | macOS | Windows |
|---|-----------|-------|-----------------|-------|---------|
| 4.1 | Auto-stop on silence (toggle) | Set toggle mode, press hotkey, stay silent >2 s | Recording auto-stops, flow proceeds to processing | | |
| 4.2 | Silence threshold effective | Whisper very quietly | Recording continues (not treated as silence if above threshold) | | |
| 4.3 | Short audio discarded | Start and stop instantly (<100 ms of audio) | Silent discard, no injection, returns to Idle | | |
| 4.4 | Speech after silence continues | Press toggle, pause, then speak | Silence timer resets on speech, recording continues | | |

---

## 5. Max Duration Enforcement

| # | Test Case | Steps | Expected Result | macOS | Windows |
|---|-----------|-------|-----------------|-------|---------|
| 5.1 | Recording stops at 60 s | Hold hotkey for 60+ seconds | Recording auto-stops at 60 s, proceeds to processing | | |
| 5.2 | Timer shown in UI | Check popover during long recording | Elapsed time counter visible, approaches 60 s | | |
| 5.3 | Near-limit warning | Record to ~55 s | Optional: visual warning that max duration is near | | |
| 5.4 | No crash on overflow | Attempt to record past 60 s | Stream stops, no crash, clean transition to Idle | | |

---

## 6. Server Unreachable Warning

| # | Test Case | Steps | Expected Result | macOS | Windows |
|---|-----------|-------|-----------------|-------|---------|
| 6.1 | Warning on launch with server down | Kill Python server, restart app | Error badge / warning indicator on tray icon, tooltip "Server unreachable" | | |
| 6.2 | Warning during operation | Stop server while app is running | Status changes to ERROR, badge indicator appears | | |
| 6.3 | Recording blocked when server down | Press hotkey while server is unreachable | Recording starts but processing fails, returns to Idle with error | | |
| 6.4 | Recovery when server comes back | Restart server, wait, click health-check | Health check passes, warning clears | | |
| 6.5 | Correct error message in popover | Check popover text when server unreachable | Shows "Server unreachable — start parakeet_server" or similar | | |

---

## 7. Transcription in Popover

| # | Test Case | Steps | Expected Result | macOS | Windows |
|---|-----------|-------|-----------------|-------|---------|
| 7.1 | Transcription text appears | Speak a sentence, release hotkey | Transcribed text appears in popover after processing | | |
| 7.2 | Timestamp shown | Check transcribed segment | Duration and/or inference time displayed | | |
| 7.3 | Auto-scroll | Transcribe multiple times | Latest transcription visible, history scrolls | | |
| 7.4 | Language label | Transcribe with explicit language set | Language code shown alongside transcription | | |

---

## 8. Text Injection at Cursor

| # | Test Case | Steps | Expected Result | macOS | Windows |
|---|-----------|-------|-----------------|-------|---------|
| 8.1 | Text injected in text editor | Open TextEdit/Notepad, press hotkey, speak, release | Transcribed text appears at cursor in the editor | | |
| 8.2 | Text injected in browser | Open a text field in Chrome/Firefox/Edge, record | Text appears in the focused text field | | |
| 8.3 | Text injected in terminal | Open a terminal with a shell prompt, record | Text appears at shell prompt (may need accessibility permissions) | | |
| 8.4 | Empty text not injected | Record silence → receive empty transcription | No injection attempted, clipboard unchanged | | |
| 8.5 | Correct keyboard layout | Type in editor with different keyboard layout | Injected text matches transcription (not layout-dependent) | | |

---

## 9. Clipboard Restore

| # | Test Case | Steps | Expected Result | macOS | Windows |
|---|-----------|-------|-----------------|-------|---------|
| 9.1 | Clipboard saved before injection | Copy "SECRET", record speech | After injection, "SECRET" is still in clipboard | | |
| 9.2 | Injection uses Paste not SetText | Check clipboard during injection | Clipboard temporarily contains transcription, then restored | | |
| 9.3 | Empty result does not touch clipboard | Copy text, record silence | Clipboard unchanged after empty discard | | |
| 9.4 | Crash does not leak clipboard | Simulate crash during injection | Original clipboard content is persisted | | |

---

## 10. Settings Persistence

| # | Test Case | Steps | Expected Result | macOS | Windows |
|---|-----------|-------|-----------------|-------|---------|
| 10.1 | Hotkey persists after restart | Change hotkey, quit app, relaunch | New hotkey is active | | |
| 10.2 | Language persists | Set language to French, restart | Language default is "fr" | | |
| 10.3 | Mode persists | Change to Toggle mode, restart | Mode stays Toggle | | |
| 10.4 | Silence threshold persists | Set threshold to 0.01, restart | Threshold remains 0.01 | | |
| 10.5 | Server URL persists | Change server URL, restart | Custom URL is retained | | |

---

## 11. Launch at Login

| # | Test Case | Steps | Expected Result | macOS | Windows |
|---|-----------|-------|-----------------|-------|---------|
| 11.1 | Toggle launch at login | Open Settings → General, enable "Launch at login" | Setting saved, login item registered | | |
| 11.2 | Disable launch at login | Uncheck "Launch at login" | Login item removed | | |
| 11.3 | App starts after login (macOS) | Enable, log out and log back in | ParakeetSTT launches automatically | | |
| 11.4 | App starts after startup (Windows) | Enable, restart Windows | App launches automatically | | |
| 11.5 | Persists across reboots | Reboot twice | Login item active after both reboots | | |

---

## 12. Quit from Tray

| # | Test Case | Steps | Expected Result | macOS | Windows |
|---|-----------|-------|-----------------|-------|---------|
| 12.1 | Quit via context menu | Right-click tray → Quit | App terminates, icon disappears | | |
| 12.2 | No zombie process | Run `ps aux | grep parakeet` after quit | No parakeet-stt process remains | | |
| 12.3 | Quit while recording | Start recording, then Quit | Recording stops cleanly, app exits | | |
| 12.4 | Quit while processing | Start recording, release, immediately Quit | Processing aborts, app exits without panic | | |

---

## Platform-Specific Notes

### macOS (Primary)
| Concern | Detail |
|---------|--------|
| Accessibility permission | Required for text injection (enigo keyboard simulation). Grant in System Settings → Privacy → Accessibility |
| Microphone permission | Required for audio capture. Grant in System Settings → Privacy → Microphone |
| Screen Recording permission | May be required by some macOS versions for enigo to work in all apps |
| Launch Agent | Autostart uses `LaunchAgent` plist (`~/Library/LaunchAgents/`) |
| No Dock icon | App uses `LSUIElement = true` in `Info.plist` |
| Tray area | Menu bar, right side (near clock / WiFi) |

### Windows (Secondary)
| Concern | Detail |
|---------|--------|
| Microphone permission | Required. Grant in Settings → Privacy & Security → Microphone |
| Antivirus | May flag keyboard simulation. Add exclusion if needed |
| Clipboard | Handled via `clipboard` crate (Win32 API) |
| Autostart | Uses Windows Registry `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` |
| Tray area | System tray, bottom-right (near clock) |
| Console window | Release builds suppress console via `#![windows_subsystem = "windows"]` |

---

## Issues & Regressions Log

| Date | Test # | Platform | Outcome | Notes |
|------|--------|----------|---------|-------|
| | | | | |

---

## Continuous Integration

The following automated checks run on every PR and push to `master`:

- **`cargo test`** — Unit tests for state machine, stubs, and audio utilities (see `src-tauri/src/`)
- **Linting** — `cargo clippy`
- **Format check** — `cargo fmt --check`

E2E tests are **manual** (require real microphone, global hotkey, GUI). Run this checklist before each release.
