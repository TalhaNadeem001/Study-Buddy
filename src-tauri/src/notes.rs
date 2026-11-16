use crate::settings::Settings;
use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Path, PathBuf};

const NOTE_EXTENSION: &str = ".md";

fn list_dir(dir: &Path) -> Vec<String> {
    match fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Find an existing entry in `dir` whose name matches `name` case-insensitively.
fn match_entry(dir: &Path, name: &str) -> Option<String> {
    let lower = name.to_lowercase();
    list_dir(dir).into_iter().find(|e| e.to_lowercase() == lower)
}

pub struct ResolvedTarget {
    pub abs_path: PathBuf,
    pub exists: bool,
}

/// Resolve where a note should be written, matching existing folders/files
/// case-insensitively so spoken lowercase names map onto Title Case paths.
/// The target always lands inside the primary note directory.
pub fn resolve_target_path(folder: &str, note: &str, settings: &Settings) -> Result<ResolvedTarget> {
    let root = settings
        .allowed_directories
        .first()
        .ok_or_else(|| anyhow!("No note directory configured. Add one in Settings first."))?;

    // Walk folder segments, reusing existing directories by case-insensitive match.
    let mut current = PathBuf::from(root);
    for seg in folder.split('/').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let matched = match_entry(&current, seg).unwrap_or_else(|| seg.to_string());
        current = current.join(matched);
    }

    // Resolve the note file name (append .md if missing).
    let note_name = {
        let trimmed = note.trim();
        if trimmed.is_empty() {
            "Inbox"
        } else {
            trimmed
        }
    };
    let file_name = if note_name.to_lowercase().ends_with(NOTE_EXTENSION) {
        note_name.to_string()
    } else {
        format!("{note_name}{NOTE_EXTENSION}")
    };
    let file_name = match_entry(&current, &file_name).unwrap_or(file_name);
    let abs_path = current.join(file_name);
    let exists = abs_path.exists();

    Ok(ResolvedTarget { abs_path, exists })
}

/// True if `target` is the same as, or nested inside, one of the allowed dirs.
pub fn is_within_allowed(target: &Path, allowed: &[String]) -> bool {
    allowed.iter().any(|dir| target.starts_with(Path::new(dir)))
}

pub fn read_note(abs_path: &Path) -> String {
    fs::read_to_string(abs_path).unwrap_or_default()
}

#[derive(Clone, Copy)]
pub enum WriteAction {
    Created,
    Appended,
}

impl WriteAction {
    pub fn as_str(self) -> &'static str {
        match self {
            WriteAction::Created => "created",
            WriteAction::Appended => "appended",
        }
    }
}

/// Create the note, or append to it with a blank-line separator.
pub fn write_note(abs_path: &Path, content: &str, exists: bool) -> Result<WriteAction> {
    if let Some(parent) = abs_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if !exists {
        let body = if content.ends_with('\n') {
            content.to_string()
        } else {
            format!("{content}\n")
        };
        fs::write(abs_path, body)?;
        return Ok(WriteAction::Created);
    }

    let current = read_note(abs_path);
    let base = if current.ends_with('\n') {
        current
    } else {
        format!("{current}\n")
    };
    let tail = if content.ends_with('\n') { "" } else { "\n" };
    fs::write(abs_path, format!("{base}\n{content}{tail}"))?;
    Ok(WriteAction::Appended)
}
