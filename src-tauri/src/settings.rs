use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// User settings. Serialized as camelCase to match the TypeScript `Settings`
/// shape exactly (the IPC boundary).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Absolute dirs the app may write notes into. The first is the default root.
    pub allowed_directories: Vec<String>,
    pub default_folder: String,
    pub default_note: String,
    pub wake_phrase: String,
    pub rewrite_style: String,
    /// OpenAI API key (falls back to the OPENAI_API_KEY env var when blank).
    #[serde(default)]
    pub openai_api_key: String,
    /// OpenAI chat model for the note rewrite.
    #[serde(default = "default_openai_model")]
    pub openai_model: String,
    /// OpenAI transcription model.
    #[serde(default = "default_transcribe_model")]
    pub transcribe_model: String,
    /// Runtime flag: is OpenAI configured? (derived, not user-set.)
    #[serde(default)]
    pub ai_configured: bool,
}

fn default_openai_model() -> String {
    "gpt-4o-mini".to_string()
}
fn default_transcribe_model() -> String {
    "gpt-transcribe".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            allowed_directories: Vec::new(),
            default_folder: String::new(),
            default_note: "Inbox".to_string(),
            wake_phrase: "hey write this note".to_string(),
            rewrite_style: "concise".to_string(),
            openai_api_key: String::new(),
            openai_model: default_openai_model(),
            transcribe_model: default_transcribe_model(),
            ai_configured: false,
        }
    }
}

impl Settings {
    /// Trim/drop-empty/de-duplicate the note directories (order preserved), and
    /// reset the runtime-only `ai_configured` flag.
    pub fn normalized(mut self) -> Self {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for dir in self.allowed_directories.drain(..) {
            let trimmed = dir.trim().to_string();
            if trimmed.is_empty() {
                continue;
            }
            if seen.insert(trimmed.clone()) {
                out.push(trimmed);
            }
        }
        self.allowed_directories = out;
        // Derived runtime flag for the UI banner (env-var keys aren't reflected here).
        self.ai_configured = !self.openai_api_key.trim().is_empty();
        if self.wake_phrase.trim().is_empty() {
            self.wake_phrase = "hey write this note".to_string();
        }
        if self.openai_model.trim().is_empty() {
            self.openai_model = default_openai_model();
        }
        if self.transcribe_model.trim().is_empty() {
            self.transcribe_model = default_transcribe_model();
        }
        self
    }
}
