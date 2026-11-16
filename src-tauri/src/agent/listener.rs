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
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

// ---- Deterministic state machine (pure, unit-tested) ------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Disabled,
    Armed,
    Recording,
}

/// Dispatch emitted when a VAD segment should be transcribed for a note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentDispatch {
    pub note: u64,
    pub index: usize,
    /// First segment of a note — strip a leading wake phrase from its transcript.
    pub strip_wake: bool,
}

/// The controller. Transitions are driven only from the single capture thread,
/// so they are inherently serialized.
pub struct Machine {
    pub state: State,
    current_note: Option<u64>,
    next_note_id: u64,
    seg_index: usize,
    first_segment: bool,
}

impl Machine {
    pub fn armed() -> Self {
        Self {
            state: State::Armed,
            current_note: None,
            next_note_id: 1,
            seg_index: 0,
            first_segment: true,
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
        self.seg_index = 0;
        self.first_segment = true;
        self.state = State::Recording;
        Some(id)
    }

    /// End in RECORDING closes the note's capture interval and rearms. Returns
    /// `(note_id, total_segments_dispatched)`.
    pub fn on_end(&mut self) -> Option<(u64, usize)> {
        if self.state != State::Recording {
            return None;
        }
        let note = self.current_note.take()?;
        let total = self.seg_index;
        self.state = State::Armed; // rearm immediately, independent of finalization
        Some((note, total))
    }

    /// A VAD segment commits a transcription unit but never ends the note. Only
    /// counts while RECORDING (segments in ARMED — e.g. the end-phrase audio or
    /// ambient speech — are ignored).
    pub fn on_segment(&mut self) -> Option<SegmentDispatch> {
        if self.state != State::Recording {
            return None;
        }
        let note = self.current_note?;
        let index = self.seg_index;
        let strip_wake = self.first_segment;
        self.seg_index += 1;
        self.first_segment = false;
        Some(SegmentDispatch {
            note,
            index,
            strip_wake,
        })
    }
}

// ---- Transcription manager (off the capture thread) -------------------------

enum MgrMsg {
    Segment {
        note: u64,
        index: usize,
        wav: Vec<u8>,
        strip_wake: bool,
    },
    Close {
        note: u64,
        total: usize,
    },
}

struct NoteBuf {
    segments: BTreeMap<usize, String>,
    received: usize,
    total: Option<usize>,
}

impl NoteBuf {
    fn new() -> Self {
        Self {
            segments: BTreeMap::new(),
            received: 0,
            total: None,
        }
    }
    fn ready(&self) -> bool {
        matches!(self.total, Some(t) if self.received >= t)
    }
}

/// Assemble a note's completed segments in capture order.
pub fn assemble(segments: &BTreeMap<usize, String>) -> String {
    segments
        .values()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Remove a leading control phrase (wake) from the first segment's transcript,
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

fn spawn_manager(app: AppHandle) -> mpsc::Sender<MgrMsg> {
    let (tx, rx) = mpsc::channel::<MgrMsg>();
    std::thread::spawn(move || {
        let mut notes: HashMap<u64, NoteBuf> = HashMap::new();
        while let Ok(msg) = rx.recv() {
            match msg {
                MgrMsg::Segment {
                    note,
                    index,
                    wav,
                    strip_wake,
                } => {
                    let settings = app.state::<Storage>().load_settings().unwrap_or_default();
                    let mut text = match openai::transcribe(&settings, &wav) {
                        Ok(t) => t.trim().to_string(),
                        Err(e) => {
                            eprintln!("[listener] transcription failed (note {note}, seg {index}): {e}");
                            String::new()
                        }
                    };
                    if strip_wake && !text.is_empty() {
                        text = strip_leading_phrase(&text, &settings.wake_phrase);
                    }
                    let buf = notes.entry(note).or_insert_with(NoteBuf::new);
                    buf.received += 1;
                    if !text.is_empty() {
                        buf.segments.insert(index, text);
                    }
                    if buf.ready() {
                        finalize(&app, note, notes.remove(&note).unwrap());
                    }
                }
                MgrMsg::Close { note, total } => {
                    let buf = notes.entry(note).or_insert_with(NoteBuf::new);
                    buf.total = Some(total);
                    if buf.ready() {
                        finalize(&app, note, notes.remove(&note).unwrap());
                    }
                }
            }
        }
    });
    tx
}

fn finalize(app: &AppHandle, _note: u64, buf: NoteBuf) {
    let transcript = assemble(&buf.segments);
    if transcript.trim().is_empty() {
        emit_state(app, "listening", Some("Listening for your wake phrase…".to_string()));
        return;
    }
    let storage = app.state::<Storage>();
    match orchestrator::process_capture(app, storage.inner(), &transcript) {
        Ok(_) => emit_state(app, "completed", None),
        Err(e) => {
            eprintln!("[listener] deliver failed: {e}");
            emit_state(app, "error", Some(format!("Failed to save note: {e}")));
        }
    }
    emit_state(app, "listening", Some("Listening for your wake phrase…".to_string()));
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
    let manager = spawn_manager(app.clone());

    let mut machine = Machine::armed();
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

        for event in detectors.feed(&chunk) {
            match event {
                DetectEvent::Wake => {
                    if machine.on_wake().is_some() {
                        emit_state(app, "capturing", Some("Recording your note…".to_string()));
                    }
                }
                DetectEvent::End => {
                    if let Some((note, total)) = machine.on_end() {
                        let _ = manager.send(MgrMsg::Close { note, total });
                        emit_state(app, "transcribing", Some("Finishing transcription…".to_string()));
                        detectors.reset();
                    }
                }
                DetectEvent::Segment(pcm) => {
                    if let Some(d) = machine.on_segment() {
                        // Half a second minimum to avoid transcribing clicks.
                        if pcm.len() >= 8000 / 2 {
                            let wav = encode_wav(&f32_to_i16(&pcm), 16_000)?;
                            let _ = manager.send(MgrMsg::Segment {
                                note: d.note,
                                index: d.index,
                                wav,
                                strip_wake: d.strip_wake,
                            });
                        } else if d.index == 0 {
                            // too-short first segment: keep the note recording anyway
                        }
                    }
                }
            }
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
    fn segments_only_count_while_recording_and_first_strips_wake() {
        let mut m = Machine::armed();
        assert_eq!(m.on_segment(), None, "segment in ARMED is ignored");
        let n = m.on_wake().unwrap();
        let s0 = m.on_segment().unwrap();
        assert_eq!(s0, SegmentDispatch { note: n, index: 0, strip_wake: true });
        let s1 = m.on_segment().unwrap();
        assert_eq!(s1, SegmentDispatch { note: n, index: 1, strip_wake: false });
    }

    #[test]
    fn end_closes_note_rearms_and_reports_total() {
        let mut m = Machine::armed();
        let n = m.on_wake().unwrap();
        m.on_segment();
        m.on_segment();
        assert_eq!(m.on_end(), Some((n, 2)));
        assert_eq!(m.state, State::Armed);
        assert_eq!(m.on_end(), None, "end without a note is ignored");
        // A segment after rearm (e.g. the end-phrase audio) is ignored.
        assert_eq!(m.on_segment(), None);
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
    fn assemble_orders_and_skips_empty() {
        let mut segs = BTreeMap::new();
        segs.insert(2usize, "third".to_string());
        segs.insert(0usize, "first".to_string());
        segs.insert(1usize, "".to_string());
        assert_eq!(assemble(&segs), "first third");
    }

    #[test]
    fn strip_leading_wake_only_at_start() {
        assert_eq!(strip_leading_phrase("Hey Atlas, remember the milk", "Hey Atlas"), "remember the milk");
        assert_eq!(strip_leading_phrase("hey atlas buy eggs", "Hey Atlas"), "buy eggs");
        // Not at the start → untouched.
        assert_eq!(strip_leading_phrase("tell hey atlas later", "Hey Atlas"), "tell hey atlas later");
    }
}
