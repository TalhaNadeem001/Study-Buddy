//! Background voice listener: wake phrase -> record -> end phrase -> transcribe ->
//! existing pipeline. Single cpal stream, local sherpa-onnx detection, a
//! deterministic state machine, and per-note batch transcription that runs off
//! the capture thread (so a new note can start while the previous one finishes).

use crate::agent::audio::{encode_wav, f32_to_i16};
use crate::agent::capture;
use crate::agent::detect::{DetectEvent, Detectors};
use crate::agent::orchestrator;
use crate::ai::{self, openai};
use crate::storage::Storage;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

// ---- Deterministic state machine (pure, unit-tested) ------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Disabled,
    Armed,
    Recording,
}

/// The controller. Transitions are driven only from the single capture thread,
/// so they are inherently serialized.
pub struct Machine {
    pub state: State,
    current_note: Option<u64>,
    next_note_id: u64,
}

impl Machine {
    pub fn armed() -> Self {
        Self {
            state: State::Armed,
            current_note: None,
            next_note_id: 1,
        }
    }

    pub fn current_note(&self) -> Option<u64> {
        self.current_note
    }

    /// Wake in ARMED starts exactly one note. Repeated wakes while RECORDING are ignored.
    pub fn on_wake(&mut self) -> Option<u64> {
        if self.state != State::Armed {
            return None;
        }
        let id = self.next_note_id;
        self.next_note_id += 1;
        self.current_note = Some(id);
        self.state = State::Recording;
        Some(id)
    }

    /// End in RECORDING closes the note and rearms immediately (independent of
    /// finalization). Returns the note id.
    pub fn on_end(&mut self) -> Option<u64> {
        if self.state != State::Recording {
            return None;
        }
        let note = self.current_note.take()?;
        self.state = State::Armed;
        Some(note)
    }
}

/// Remove a leading control phrase (wake) from the transcript,
/// anchored at the start only — not a global replace.
pub fn strip_leading_phrase(text: &str, phrase: &str) -> String {
    let t = text.trim_start();
    let lead: String = t
        .chars()
        .take_while(|c| c.is_alphanumeric() || c.is_whitespace() || *c == ',' || *c == '\'')
        .collect();
    let norm = |s: &str| {
        s.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let phrase_n = norm(phrase);
    let lead_n = norm(&lead);
    if lead_n.starts_with(&phrase_n) {
        // Drop the phrase's words from the front of the raw text.
        let words_to_drop = phrase_n.split_whitespace().count();
        let mut dropped = 0;
        let mut out = String::new();
        let mut in_word = false;
        for c in t.chars() {
            if dropped >= words_to_drop {
                out.push(c);
            } else if c.is_whitespace() {
                if in_word {
                    in_word = false;
                    dropped += 1;
                }
            } else {
                in_word = true;
            }
        }
        out.trim().to_string()
    } else {
        text.trim().to_string()
    }
}

/// Remove a trailing control phrase (end) from the transcript, anchored at the end.
pub fn strip_trailing_phrase(text: &str, phrase: &str) -> String {
    let t = text.trim_end();
    let words = |s: &str| {
        s.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .map(|w| w.to_string())
            .collect::<Vec<_>>()
    };
    let phrase_words = words(phrase);
    let text_words = words(t);
    if phrase_words.is_empty() || text_words.len() < phrase_words.len() {
        return t.to_string();
    }
    let tail = &text_words[text_words.len() - phrase_words.len()..];
    if tail != phrase_words.as_slice() {
        return t.to_string();
    }
    // Keep the raw text up to the end of the (keep)-th word.
    let keep = text_words.len() - phrase_words.len();
    let mut counted = 0;
    let mut end_byte = 0;
    let mut in_word = false;
    for (i, c) in t.char_indices() {
        if c.is_whitespace() {
            in_word = false;
        } else {
            if !in_word {
                in_word = true;
                counted += 1;
                if counted > keep {
                    break;
                }
            }
            end_byte = i + c.len_utf8();
        }
    }
    t[..end_byte].trim().to_string()
}

/// Transcribe the whole note clip, strip the wake (leading) and end (trailing)
/// phrases, and deliver it to the existing pipeline. Runs on its own thread so a
/// new note can start while this finishes. Errors are surfaced to the UI — never
/// a silent reset.
fn finalize_whole(app: AppHandle, wav: Vec<u8>, wake: String, stop: String) {
    let settings = app.state::<Storage>().load_settings().unwrap_or_default();
    let raw = match openai::transcribe(&settings, &wav) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[listener] transcription failed: {e}");
            emit_state(&app, "error", Some(format!("Transcription failed: {e}")));
            emit_state(&app, "listening", Some("Listening for your wake phrase…".to_string()));
            return;
        }
    };
    let stripped = strip_trailing_phrase(&strip_leading_phrase(&raw, &wake), &stop);
    let transcript = stripped.trim();
    if transcript.is_empty() {
        emit_state(&app, "listening", Some("Nothing heard — listening again…".to_string()));
        return;
    }
    let storage = app.state::<Storage>();
    match orchestrator::process_capture(&app, storage.inner(), transcript) {
        Ok(_) => emit_state(&app, "completed", None),
        Err(e) => {
            eprintln!("[listener] deliver failed: {e}");
            emit_state(&app, "error", Some(format!("Failed to save note: {e}")));
        }
    }
    emit_state(&app, "listening", Some("Listening for your wake phrase…".to_string()));
}

// ---- Driver (capture thread) ------------------------------------------------

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct VoiceState {
    state: String,
    message: Option<String>,
}

fn emit_state(app: &AppHandle, state: &str, message: Option<String>) {
    let _ = app.emit(
        "agent://voice-state",
        VoiceState {
            state: state.to_string(),
            message,
        },
    );
    update_overlay(app, state);
}

/// Show/position/hide the always-on-top "listening" overlay to match the voice
/// phase. Display-only — the agent owns this window; its webview just renders
/// whatever voice-state it receives. Only active phases are shown; the idle
/// "waiting for the wake phrase" state keeps the overlay hidden.
fn update_overlay(app: &AppHandle, state: &str) {
    let Some(window) = app.get_webview_window("overlay") else {
        return;
    };
    match state {
        "capturing" | "transcribing" | "writing" => {
            // Note: don't call set_always_on_top here — on macOS it resets the
            // native window level, undoing the elevated level/collection behavior
            // that lets the overlay float over fullscreen apps (set once at
            // startup in lib.rs::float_over_fullscreen). The window is already
            // created alwaysOnTop, and that behavior persists across hide/show.
            position_bottom_right(&window);
            let _ = window.show();
        }
        "completed" => {
            // Keep "Saved ✓" on screen briefly, then dismiss it.
            let app = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(1500));
                if let Some(w) = app.get_webview_window("overlay") {
                    let _ = w.hide();
                }
            });
        }
        _ => {
            // idle / listening / error — nothing active to show.
            let _ = window.hide();
        }
    }
}

/// Pin the overlay to the bottom-right corner of the monitor it sits on.
fn position_bottom_right(window: &tauri::WebviewWindow) {
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        return;
    };
    let Ok(wsize) = window.outer_size() else {
        return;
    };
    let scale = monitor.scale_factor();
    let msize = monitor.size();
    let mpos = monitor.position();
    let margin_x = (16.0 * scale) as i32;
    let margin_y = (48.0 * scale) as i32; // lifted higher off the bottom edge
    let x = mpos.x + msize.width as i32 - wsize.width as i32 - margin_x;
    let y = mpos.y + msize.height as i32 - wsize.height as i32 - margin_y;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// Resolve the models directory in both bundled and dev runs.
fn models_dir(app: &AppHandle) -> PathBuf {
    if let Ok(res) = app.path().resource_dir() {
        for cand in [res.join("models"), res.join("resources/models")] {
            if cand.join("kws/encoder.onnx").exists() {
                return cand;
            }
        }
    }
    // Dev fallback: the repo's src-tauri/resources/models.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/models")
}

/// Spawn the listener (idempotent while already running).
pub fn start_listening(app: AppHandle, enabled: Arc<AtomicBool>, running: Arc<AtomicBool>) {
    if running.swap(true, Ordering::SeqCst) {
        return;
    }
    enabled.store(true, Ordering::SeqCst);

    std::thread::spawn(move || {
        if let Err(e) = run_loop(&app, &enabled) {
            emit_state(&app, "error", Some(e.to_string()));
            eprintln!("[listener] error: {e}");
        } else {
            emit_state(&app, "idle", None);
        }
        enabled.store(false, Ordering::SeqCst);
        running.store(false, Ordering::SeqCst);
    });
}

fn run_loop(app: &AppHandle, enabled: &Arc<AtomicBool>) -> anyhow::Result<()> {
    let settings = app.state::<Storage>().load_settings()?;
    if ai::api_key(&settings).is_err() {
        return Err(anyhow::anyhow!(
            "No OpenAI API key set. Add one in Settings to enable voice."
        ));
    }

    let dir = models_dir(app);
    let cap = capture::open()?; // owns the single cpal stream on THIS thread
    let mut detectors =
        Detectors::load(&dir, cap.sample_rate, &settings.wake_phrase, &settings.stop_phrase)?;

    let mut machine = Machine::armed();
    // The whole note's audio (16 kHz) accumulated while RECORDING.
    let mut note_audio: Vec<f32> = Vec::new();
    const MAX_NOTE_SAMPLES: usize = 16_000 * 180; // ~3 min safety cap

    emit_state(app, "listening", Some("Listening for your wake phrase…".to_string()));

    while enabled.load(Ordering::SeqCst) {
        let chunk = match cap.rx.recv_timeout(Duration::from_millis(200)) {
            Ok(c) => c,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        };

        if cap.overflow.swap(false, Ordering::Relaxed) {
            emit_state(app, "error", Some("Audio capture interrupted (overflow)".to_string()));
        }

        let (r16, events) = detectors.feed(&chunk);
        let mut ended = false;
        for event in events {
            match event {
                DetectEvent::Wake => {
                    if machine.on_wake().is_some() {
                        note_audio.clear();
                        emit_state(app, "capturing", Some("Recording your note…".to_string()));
                    }
                }
                DetectEvent::End => {
                    if machine.on_end().is_some() {
                        ended = true;
                        let wav = encode_wav(&f32_to_i16(&note_audio), 16_000)?;
                        note_audio.clear();
                        let app2 = app.clone();
                        let wake = settings.wake_phrase.clone();
                        let stop = settings.stop_phrase.clone();
                        std::thread::spawn(move || finalize_whole(app2, wav, wake, stop));
                        emit_state(app, "transcribing", Some("Finishing transcription…".to_string()));
                        detectors.reset();
                    }
                }
            }
        }

        // Accumulate after processing events, so the wake chunk's tail is captured
        // (and later stripped from the text). Skip the end chunk.
        if machine.state == State::Recording && !ended && note_audio.len() < MAX_NOTE_SAMPLES {
            note_audio.extend_from_slice(&r16);
        }
    }

    drop(cap); // stop the stream
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wake_starts_one_note_and_repeated_wake_is_ignored() {
        let mut m = Machine::armed();
        let n = m.on_wake().expect("wake starts a note");
        assert_eq!(m.state, State::Recording);
        assert_eq!(m.on_wake(), None, "repeated wake while recording is ignored");
        assert_eq!(m.current_note(), Some(n));
    }

    #[test]
    fn end_closes_note_and_rearms() {
        let mut m = Machine::armed();
        let n = m.on_wake().unwrap();
        assert_eq!(m.on_end(), Some(n));
        assert_eq!(m.state, State::Armed);
        assert_eq!(m.on_end(), None, "end without a note is ignored");
    }

    #[test]
    fn second_note_gets_a_new_id_while_first_may_still_finalize() {
        let mut m = Machine::armed();
        let a = m.on_wake().unwrap();
        m.on_end();
        let b = m.on_wake().unwrap();
        assert_ne!(a, b, "each note gets a stable, distinct id");
    }

    #[test]
    fn strip_leading_wake_only_at_start() {
        assert_eq!(strip_leading_phrase("Hey Atlas, remember the milk", "Hey Atlas"), "remember the milk");
        assert_eq!(strip_leading_phrase("hey atlas buy eggs", "Hey Atlas"), "buy eggs");
        // Not at the start → untouched.
        assert_eq!(strip_leading_phrase("tell hey atlas later", "Hey Atlas"), "tell hey atlas later");
    }

    #[test]
    fn strip_trailing_end_only_at_end() {
        assert_eq!(strip_trailing_phrase("buy milk and eggs Atlas save note", "Atlas, save note"), "buy milk and eggs");
        assert_eq!(strip_trailing_phrase("remember this. atlas save note.", "Atlas, save note"), "remember this.");
        // Not at the end → untouched.
        assert_eq!(strip_trailing_phrase("atlas save note was the phrase", "Atlas, save note"), "atlas save note was the phrase");
    }
}
