//! AI Orchestrator — turns a transcript into a stored, written note.
//!
//! Two entry points share the rewrite + index steps:
//! - `process_transcript` (the `create_note` command): writes into the
//!   settings-configured directory (`allowedDirectories[0]`, Obsidian-friendly),
//!   indexing into the `notes` table. Kept from Phase 1.
//! - `process_capture` (voice pipeline + `capture_text`): appends to ONE
//!   hardcoded file (`~/Documents/StudyBuddy/notes.md`), records a row in
//!   `captures` (raw transcript + written note) for the Voice tab, and mirrors
//!   the file into `notes` for search.

use crate::notes;
use crate::settings::Settings;
use crate::storage::{CaptureRecord, Storage};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteInput {
    pub transcript: String,
    #[serde(default)]
    pub folder: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteResult {
    pub id: i64,
    pub path: String,
    pub title: String,
    pub action: String,
    pub content: String,
}

/// Settings-path write (Phase 1 create_note): resolve → guard → rewrite → write → index.
pub fn process_transcript(
    app: &AppHandle,
    storage: &Storage,
    input: CreateNoteInput,
) -> Result<CreateNoteResult> {
    let transcript = input.transcript.trim();
    if transcript.is_empty() {
        return Err(anyhow!("Nothing to write — the transcript is empty."));
    }

    let settings: Settings = storage.load_settings()?;
    if settings.allowed_directories.is_empty() {
        return Err(anyhow!(
            "No note directory configured. Add one in Settings first."
        ));
    }

    let folder = input.folder.unwrap_or_else(|| settings.default_folder.clone());
    let note = input.note.unwrap_or_else(|| settings.default_note.clone());

    let target = notes::resolve_target_path(&folder, &note, &settings)?;
    if !notes::is_within_allowed(&target.abs_path, &settings.allowed_directories) {
        return Err(anyhow!(
            "Refusing to write outside allowed directories: {}",
            target.abs_path.display()
        ));
    }

    let existing = if target.exists {
        Some(notes::read_note(&target.abs_path))
    } else {
        None
    };

    let content = crate::ai::rewrite(&settings, transcript, existing.as_deref())?;
    if content.trim().is_empty() {
        return Err(anyhow!("Nothing to write after cleanup."));
    }

    let action = notes::write_note(&target.abs_path, &content, target.exists)?;

    let path_str = target.abs_path.to_string_lossy().to_string();
    let title = target
        .abs_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let full_body = notes::read_note(&target.abs_path);
    let id = storage.upsert_note(&path_str, &title, &full_body)?;

    let result = CreateNoteResult {
        id,
        path: path_str,
        title,
        action: action.as_str().to_string(),
        content,
    };
    let _ = app.emit("agent://note-created", &result);
    Ok(result)
}

/// Capture path (voice + manual text box): rewrite → append the hardcoded file →
/// record a capture → mirror into `notes` → emit events.
pub fn process_capture(app: &AppHandle, storage: &Storage, transcript: &str) -> Result<CaptureRecord> {
    let text = transcript.trim();
    if text.is_empty() {
        return Err(anyhow!("Nothing to write — the transcript is empty."));
    }

    let settings = storage.load_settings()?;
    let mut content = crate::ai::rewrite(&settings, text, None)?;
    if content.trim().is_empty() {
        content = text.to_string();
    }

    let path = capture_file_path(app)?;
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    append_capture_file(&path, &content, &timestamp)?;

    let path_str = path.to_string_lossy().to_string();
    let full_body = std::fs::read_to_string(&path).unwrap_or_default();
    // Mirror the growing file into `notes` so the Notes tab + FTS search stay useful.
    let _ = storage.upsert_note(&path_str, "Study Buddy Notes", &full_body);

    let capture = storage.insert_capture(text, &content, &path_str)?;
    let _ = app.emit("agent://capture-created", &capture);
    let _ = app.emit("agent://note-created", &capture);
    Ok(capture)
}

/// Hardcoded, for now: `~/Documents/StudyBuddy/notes.md`.
fn capture_file_path(app: &AppHandle) -> Result<std::path::PathBuf> {
    let docs = app
        .path()
        .document_dir()
        .map_err(|e| anyhow!("Cannot resolve the Documents directory: {e}"))?;
    Ok(docs.join("StudyBuddy").join("notes.md"))
}

fn append_capture_file(path: &Path, content: &str, timestamp: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut body = if path.exists() {
        std::fs::read_to_string(path).unwrap_or_default()
    } else {
        String::from("# Study Buddy Notes\n")
    };
    if !body.ends_with('\n') {
        body.push('\n');
    }
    body.push_str(&format!("\n## {timestamp}\n\n{content}\n"));
    std::fs::write(path, body)?;
    Ok(())
}
