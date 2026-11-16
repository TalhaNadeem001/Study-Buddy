use crate::agent::audio;
use crate::agent::orchestrator::{self, CreateNoteInput, CreateNoteResult};
use crate::agent::{self, AgentStatus};
use crate::settings::Settings;
use crate::storage::{CaptureRecord, NoteRecord, Storage};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

/// Shared flags controlling the wake-word listener thread.
#[derive(Default)]
pub struct VoiceControl {
    pub enabled: Arc<AtomicBool>,
    pub running: Arc<AtomicBool>,
}

/// Run `work` on a dedicated OS thread and wait for it. The voice/AI paths use
/// `reqwest::blocking`, which panics inside a tokio runtime — Tauri may run sync
/// commands on the async runtime, so we isolate the blocking work on our own thread.
fn run_blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> anyhow::Result<T> + Send + 'static,
{
    std::thread::spawn(work)
        .join()
        .map_err(|_| "worker thread panicked".to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_settings(storage: State<Storage>) -> Result<Settings, String> {
    storage.load_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_settings(storage: State<Storage>, settings: Settings) -> Result<Settings, String> {
    storage.save_settings(&settings).map_err(|e| e.to_string())
}

/// Settings-path write (Obsidian-friendly). Kept from Phase 1; the UI's main
/// flow now uses `capture_text`/voice instead.
#[tauri::command]
pub fn create_note(app: AppHandle, input: CreateNoteInput) -> Result<CreateNoteResult, String> {
    run_blocking(move || {
        let storage = app.state::<Storage>();
        orchestrator::process_transcript(&app, storage.inner(), input)
    })
}

/// Capture path used by the manual text box (and shared with the voice pipeline):
/// rewrite → append the hardcoded file → record a capture.
#[tauri::command]
pub fn capture_text(app: AppHandle, transcript: String) -> Result<CaptureRecord, String> {
    run_blocking(move || {
        let storage = app.state::<Storage>();
        orchestrator::process_capture(&app, storage.inner(), &transcript)
    })
}

#[tauri::command]
pub fn list_notes(storage: State<Storage>) -> Result<Vec<NoteRecord>, String> {
    storage.list_notes(200).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_notes(storage: State<Storage>, query: String) -> Result<Vec<NoteRecord>, String> {
    storage.search_notes(&query).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_captures(storage: State<Storage>) -> Result<Vec<CaptureRecord>, String> {
    storage.list_captures(200).map_err(|e| e.to_string())
}

/// Start/stop the wake-word listener.
#[tauri::command]
pub fn set_listening(app: AppHandle, voice: State<VoiceControl>, enabled: bool) -> Result<(), String> {
    if enabled {
        audio::start_listening(app, voice.enabled.clone(), voice.running.clone());
    } else {
        voice.enabled.store(false, Ordering::SeqCst);
    }
    Ok(())
}

#[tauri::command]
pub fn get_listening(voice: State<VoiceControl>) -> bool {
    voice.running.load(Ordering::SeqCst)
}

#[tauri::command]
pub fn agent_status() -> Result<AgentStatus, String> {
    Ok(agent::snapshot())
}
