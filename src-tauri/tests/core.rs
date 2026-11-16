//! Headless tests for the storage + AI + note-writing core (no GUI needed).

use std::fs;
use study_buddy_ai_lib::agent::audio::encode_wav;
use study_buddy_ai_lib::ai::local_cleanup;
use study_buddy_ai_lib::ai::openai::{system_prompt, user_prompt};
use study_buddy_ai_lib::notes::{resolve_target_path, write_note};
use study_buddy_ai_lib::settings::Settings;
use study_buddy_ai_lib::storage::Storage;

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("sb-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn local_cleanup_strips_filler_and_capitalizes() {
    let out = local_cleanup("um i think like this is uh really important , you know");
    assert!(out.starts_with('I'), "should capitalize: {out}");
    assert!(out.contains("important"), "keeps content: {out}");
    assert!(!out.to_lowercase().contains(" um "), "drops filler: {out}");
    assert!(!out.contains(" ,"), "fixes spacing before punctuation: {out}");
}

#[test]
fn storage_notes_roundtrip_and_fts_search() {
    let dir = temp_dir("db");
    let storage = Storage::new(&dir.join("test.db")).unwrap();

    let id = storage
        .upsert_note(
            "/tmp/vault/Inbox.md",
            "Inbox",
            "Photosynthesis converts light into chemical energy",
        )
        .unwrap();
    assert!(id > 0);

    assert_eq!(storage.list_notes(10).unwrap().len(), 1);

    let hits = storage.search_notes("photosynthesis").unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "Inbox");

    // Prefix search works too.
    assert_eq!(storage.search_notes("photo").unwrap().len(), 1);
    // No false positives.
    assert!(storage.search_notes("zzzznomatch").unwrap().is_empty());

    // Upsert on the same path updates rather than duplicates.
    storage
        .upsert_note("/tmp/vault/Inbox.md", "Inbox", "Mitochondria are the powerhouse")
        .unwrap();
    assert_eq!(storage.list_notes(10).unwrap().len(), 1);
    assert_eq!(storage.search_notes("mitochondria").unwrap().len(), 1);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn resolve_and_write_notes_case_insensitively() {
    let dir = temp_dir("vault");
    let settings = Settings {
        allowed_directories: vec![dir.to_string_lossy().to_string()],
        ..Settings::default()
    };

    // Create.
    let target = resolve_target_path("Knowledge/Bio", "Cells", &settings).unwrap();
    assert!(!target.exists);
    assert_eq!(write_note(&target.abs_path, "First line", target.exists).unwrap().as_str(), "created");
    assert!(target.abs_path.exists());

    // Re-resolve with different casing -> matches the same file, then appends.
    let again = resolve_target_path("knowledge/bio", "cells", &settings).unwrap();
    assert!(again.exists, "case-insensitive match should find the existing note");
    assert_eq!(write_note(&again.abs_path, "Second line", again.exists).unwrap().as_str(), "appended");

    let body = fs::read_to_string(&again.abs_path).unwrap();
    assert!(body.contains("First line") && body.contains("Second line"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn wav_encoding_has_riff_header_and_data() {
    // 16 kHz mono, 1600 samples = 0.1 s.
    let samples: Vec<i16> = (0..1600).map(|i| ((i % 100) as i16 - 50) * 100).collect();
    let wav = encode_wav(&samples, 16_000).unwrap();
    assert_eq!(&wav[0..4], b"RIFF", "WAV should start with RIFF");
    assert_eq!(&wav[8..12], b"WAVE", "WAV should declare WAVE format");
    // 44-byte header + 2 bytes/sample.
    assert_eq!(wav.len(), 44 + samples.len() * 2);
}

#[test]
fn kws_and_vad_models_load() {
    // Exercises the real sherpa-onnx KWS keyword encoding (the thing that failed
    // when the @label had spaces) + VAD load. Skips if models aren't fetched.
    let models = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/models");
    if !models.join("kws/encoder.onnx").exists() {
        eprintln!("skipping kws_and_vad_models_load: run scripts/fetch-models.sh first");
        return;
    }
    // Arbitrary phrases exercise sherpa's runtime BPE tokenization.
    let result =
        study_buddy_ai_lib::agent::detect::Detectors::load(&models, 48_000, "hello world", "goodbye now");
    assert!(result.is_ok(), "detectors should load custom phrases: {:?}", result.err());
}

#[test]
fn prompt_builders_reflect_style_and_append_context() {
    let concise = system_prompt("concise");
    assert!(concise.contains("tight, clear notes"));
    let verbatim = system_prompt("verbatim");
    assert!(verbatim.contains("almost exactly"));

    let fresh = user_prompt("photosynthesis is cool", None);
    assert!(fresh.contains("dictated this for a new note"));

    let appended = user_prompt("and mitochondria too", Some("Existing note body"));
    assert!(appended.contains("already contains"));
    assert!(appended.contains("Write only the new content"));
}
