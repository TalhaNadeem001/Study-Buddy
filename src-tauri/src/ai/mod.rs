pub mod openai;

use crate::settings::Settings;
use anyhow::{anyhow, Result};
use regex::Regex;

/// Resolve the OpenAI API key: the Settings field first, then the
/// `OPENAI_API_KEY` env var. Errs if neither is set.
pub fn api_key(settings: &Settings) -> Result<String> {
    let from_settings = settings.openai_api_key.trim();
    if !from_settings.is_empty() {
        return Ok(from_settings.to_string());
    }
    if let Ok(env) = std::env::var("OPENAI_API_KEY") {
        if !env.trim().is_empty() {
            return Ok(env.trim().to_string());
        }
    }
    Err(anyhow!("No OpenAI API key set. Add one in Settings."))
}

/// Turn a transcript into a clean note: OpenAI when a key is configured,
/// otherwise local cleanup. Falls back to local cleanup if OpenAI errors, so a
/// note is never lost.
pub fn rewrite(settings: &Settings, transcript: &str, existing: Option<&str>) -> Result<String> {
    let text = transcript.trim();
    if text.is_empty() {
        return Ok(String::new());
    }
    if api_key(settings).is_ok() {
        match openai::chat_rewrite(settings, text, existing) {
            Ok(out) => Ok(out),
            Err(e) => {
                eprintln!("[ai] OpenAI rewrite failed, using local cleanup: {e}");
                Ok(local_cleanup(text))
            }
        }
    } else {
        Ok(local_cleanup(text))
    }
}

/// Offline cleanup: strip filler words, collapse whitespace, fix spacing before
/// punctuation, capitalize. Ported from the old Express `ai.service.ts`.
pub fn local_cleanup(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }

    let filler = Regex::new(r"(?i)\b(um+|uh+|like|you know|i mean|sort of|kind of)\b").unwrap();
    let mut cleaned = filler.replace_all(text, "").to_string();

    let whitespace = Regex::new(r"\s+").unwrap();
    cleaned = whitespace.replace_all(&cleaned, " ").to_string();

    let before_punct = Regex::new(r"\s+([,.!?])").unwrap();
    cleaned = before_punct.replace_all(&cleaned, "$1").to_string();

    cleaned = cleaned.trim().to_string();
    if let Some(first) = cleaned.chars().next() {
        let rest = &cleaned[first.len_utf8()..];
        cleaned = first.to_uppercase().collect::<String>() + rest;
    }
    cleaned
}
