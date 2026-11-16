//! OpenAI HTTP client (blocking reqwest). Transcription (multipart audio) and
//! chat rewrite. Prompts are ported from the old Express `ai.service.ts`.
//!
//! NOTE: `reqwest::blocking` must not run inside a tokio runtime, so callers run
//! these on a dedicated OS thread (the voice pipeline thread, or the thread the
//! `capture_text`/`create_note` commands spawn).

use crate::ai::api_key;
use crate::settings::Settings;
use anyhow::{anyhow, Result};

const TRANSCRIBE_URL: &str = "https://api.openai.com/v1/audio/transcriptions";
const CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";

/// Transcribe a WAV clip via OpenAI (`gpt-transcribe` by default).
pub fn transcribe(settings: &Settings, wav: &[u8]) -> Result<String> {
    let key = api_key(settings)?;
    let model = {
        let m = settings.transcribe_model.trim();
        if m.is_empty() { "gpt-transcribe" } else { m }
    };

    let part = reqwest::blocking::multipart::Part::bytes(wav.to_vec())
        .file_name("audio.wav")
        .mime_str("audio/wav")?;
    let form = reqwest::blocking::multipart::Form::new()
        .text("model", model.to_string())
        .part("file", part);

    let resp = reqwest::blocking::Client::new()
        .post(TRANSCRIBE_URL)
        .bearer_auth(key)
        .multipart(form)
        .send()?;

    let status = resp.status();
    let body = resp.text()?;
    if !status.is_success() {
        return Err(anyhow!("OpenAI transcription error {}: {}", status.as_u16(), body));
    }
    let value: serde_json::Value = serde_json::from_str(&body)?;
    Ok(value
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .trim()
        .to_string())
}

/// Rewrite a transcript into a clean note via OpenAI chat completions.
pub fn chat_rewrite(settings: &Settings, transcript: &str, existing: Option<&str>) -> Result<String> {
    let key = api_key(settings)?;
    let model = {
        let m = settings.openai_model.trim();
        if m.is_empty() { "gpt-4o-mini" } else { m }
    };

    let payload = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system_prompt(&settings.rewrite_style) },
            { "role": "user", "content": user_prompt(transcript, existing) },
        ],
    });

    let resp = reqwest::blocking::Client::new()
        .post(CHAT_URL)
        .bearer_auth(key)
        .json(&payload)
        .send()?;

    let status = resp.status();
    let body = resp.text()?;
    if !status.is_success() {
        return Err(anyhow!("OpenAI chat error {}: {}", status.as_u16(), body));
    }
    let value: serde_json::Value = serde_json::from_str(&body)?;
    let out = value["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();
    if out.is_empty() {
        return Err(anyhow!("OpenAI returned empty content"));
    }
    Ok(out)
}

const FORMAT_RULES: &str = "Output only the note body in Markdown — no title, no preamble, no code fences, no YAML \
frontmatter. Use '-' for bullet lists and '####' for a heading only when the note clearly has \
distinct parts. When continuing an existing note, match its terminology and voice, don't repeat \
points already present, and phrase the new content so it fits coherently.";

fn style_instructions(style: &str) -> &'static str {
    match style {
        "light" => "Lightly clean up the dictation: keep the speaker's words and meaning, remove filler \
            (\"um\", \"like\", false starts), fix grammar and punctuation, and break it into readable \
            paragraphs or bullets. Do not summarize or reorganize.",
        "verbatim" => "Write down what was said almost exactly, with only minimal cleanup (capitalization \
            and punctuation). Do not rephrase or restructure.",
        // "concise" (default)
        _ => "Rewrite the dictation into tight, clear notes — do not transcribe it. Rephrase freely, cut \
            repetition, filler, and tangents, and restructure so the point lands fast. You may add a short \
            clarifying line when it genuinely helps. Stay true to the speaker's intent and key points; never \
            invent claims they'd disagree with.",
    }
}

pub fn system_prompt(style: &str) -> String {
    format!(
        "You turn a person's spoken dictation into a note for their personal knowledge base.\n\n{}\n\n{}",
        style_instructions(style),
        FORMAT_RULES
    )
}

pub fn user_prompt(transcript: &str, existing: Option<&str>) -> String {
    match existing {
        Some(content) if !content.trim().is_empty() => format!(
            "This note already contains the following content:\n\"\"\"\n{content}\n\"\"\"\n\n\
             The speaker just dictated this to add to it:\n\"\"\"\n{transcript}\n\"\"\"\n\n\
             Write only the new content to append."
        ),
        _ => format!("The speaker dictated this for a new note:\n\"\"\"\n{transcript}\n\"\"\""),
    }
}
