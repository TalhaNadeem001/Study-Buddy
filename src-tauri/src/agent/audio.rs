//! Audio Engine + wake-phrase listener.
//!
//! Captures the microphone with `cpal`, segments speech with an energy-based
//! VAD, transcribes each utterance with OpenAI, and runs a wake/stop-phrase
//! state machine (idle → listening → capturing) mirroring the original Web
//! Speech design. On finishing a capture it hands the text to the orchestrator
//! (rewrite + save + record). Runs on its own OS thread so the blocking HTTP
//! calls never touch a tokio runtime. OpenAI accepts any sample rate, so no
//! resampling is needed.

use crate::agent::orchestrator;
use crate::ai::{self, openai};
use crate::settings::Settings;
use crate::storage::Storage;
use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct VoiceState {
    state: String,
    message: Option<String>,
}

const STOP_PHRASES: [&str; 5] = ["end note", "stop note", "thats it", "that s it", "end of note"];
const SILENCE_RMS: f64 = 450.0; // below this a frame counts as silence
const UTTERANCE_SILENCE_MS: u64 = 700; // trailing silence that ends one utterance
const CAPTURE_TIMEOUT_MS: u128 = 6000; // silence while capturing → finalize the note
const MAX_UTTERANCE_MS: u64 = 30_000; // hard cap per utterance

pub struct AudioEngine;

impl AudioEngine {
    pub fn start() {
        eprintln!("[agent] AudioEngine: ready (wake-phrase listener available on demand)");
    }
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

/// Spawn the capture loop (idempotent while already running).
pub fn start_listening(app: AppHandle, enabled: Arc<AtomicBool>, running: Arc<AtomicBool>) {
    if running.swap(true, Ordering::SeqCst) {
        return;
    }
    enabled.store(true, Ordering::SeqCst);

    std::thread::spawn(move || {
        if let Err(e) = run_loop(&app, &enabled) {
            emit_state(&app, "error", Some(e.to_string()));
            eprintln!("[agent] voice loop error: {e}");
        } else {
            emit_state(&app, "idle", None);
        }
        enabled.store(false, Ordering::SeqCst);
        running.store(false, Ordering::SeqCst);
    });
}

fn run_loop(app: &AppHandle, enabled: &Arc<AtomicBool>) -> Result<()> {
    let settings = app.state::<Storage>().load_settings()?;
    if ai::api_key(&settings).is_err() {
        return Err(anyhow!("No OpenAI API key set. Add one in Settings to enable voice."));
    }
    let wake = normalize(&settings.wake_phrase);
    if wake.is_empty() {
        return Err(anyhow!("No wake phrase set in Settings."));
    }

    let (device, config, sample_format, channels) = open_input()?;
    let sample_rate = config.sample_rate.0;
    let (tx, rx) = mpsc::channel::<Vec<i16>>();
    let stream = build_stream(&device, &config, sample_format, channels, tx)?;
    stream
        .play()
        .map_err(|e| anyhow!("Failed to start microphone: {e}"))?;

    let listening_msg = || Some(format!("Say “{}”, then dictate", settings.wake_phrase));
    emit_state(app, "listening", listening_msg());

    let mut capturing = false;
    let mut note = String::new();
    let mut last_activity = Instant::now();

    // VAD accumulation.
    let mut utterance: Vec<i16> = Vec::new();
    let mut in_speech = false;
    let mut silence_ms: u64 = 0;
    let max_samples = (MAX_UTTERANCE_MS * sample_rate as u64 / 1000) as usize;

    while enabled.load(Ordering::SeqCst) {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(chunk) => {
                let chunk_ms = (chunk.len() as u64 * 1000) / sample_rate.max(1) as u64;
                if rms(&chunk) > SILENCE_RMS {
                    in_speech = true;
                    silence_ms = 0;
                    utterance.extend_from_slice(&chunk);
                    last_activity = Instant::now();
                } else if in_speech {
                    silence_ms += chunk_ms;
                    utterance.extend_from_slice(&chunk);
                    if silence_ms >= UTTERANCE_SILENCE_MS || utterance.len() >= max_samples {
                        let audio = std::mem::take(&mut utterance);
                        in_speech = false;
                        silence_ms = 0;
                        handle_utterance(app, &settings, &audio, sample_rate, &wake, &mut capturing, &mut note)?;
                        if capturing {
                            last_activity = Instant::now();
                        } else {
                            emit_state(app, "listening", listening_msg());
                        }
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        // In capturing mode, finalize the note after a stretch of silence.
        if capturing
            && last_activity.elapsed().as_millis() > CAPTURE_TIMEOUT_MS
            && !note.trim().is_empty()
        {
            finalize_note(app, &note)?;
            note.clear();
            capturing = false;
            emit_state(app, "listening", listening_msg());
        }
    }

    drop(stream);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_utterance(
    app: &AppHandle,
    settings: &Settings,
    audio: &[i16],
    sample_rate: u32,
    wake: &str,
    capturing: &mut bool,
    note: &mut String,
) -> Result<()> {
    // Ignore clips shorter than ~0.25 s.
    if audio.len() < (sample_rate as usize) / 4 {
        return Ok(());
    }

    emit_state(app, "transcribing", None);
    let wav = encode_wav(audio, sample_rate)?;
    let text = match openai::transcribe(settings, &wav) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[agent] transcription failed: {e}");
            return Ok(());
        }
    };
    let norm = normalize(&text);
    if norm.is_empty() {
        return Ok(());
    }

    if *capturing {
        if STOP_PHRASES.iter().any(|p| norm.contains(p)) {
            if !note.trim().is_empty() {
                finalize_note(app, note)?;
            }
            note.clear();
            *capturing = false;
        } else {
            note.push_str(text.trim());
            note.push(' ');
            emit_state(app, "capturing", Some("Dictating… (pause or say “end note”)".to_string()));
        }
    } else if norm.contains(wake) {
        *capturing = true;
        // Keep anything the user said after the wake phrase in the same breath.
        let after = strip_through(&text, &settings.wake_phrase);
        if !after.trim().is_empty() {
            note.push_str(after.trim());
            note.push(' ');
        }
        emit_state(app, "capturing", Some("Dictating… (pause or say “end note”)".to_string()));
    }
    // else: not capturing and no wake phrase → ignore the utterance.
    Ok(())
}

fn finalize_note(app: &AppHandle, note: &str) -> Result<()> {
    emit_state(app, "writing", None);
    let storage = app.state::<Storage>();
    orchestrator::process_capture(app, storage.inner(), note.trim())?;
    Ok(())
}

// ---- audio helpers ----------------------------------------------------------

fn open_input() -> Result<(cpal::Device, cpal::StreamConfig, cpal::SampleFormat, u16)> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow!("No microphone (input device) found"))?;
    let supported = device
        .default_input_config()
        .map_err(|e| anyhow!("No default input config: {e}"))?;
    let sample_format = supported.sample_format();
    let channels = supported.channels();
    let config: cpal::StreamConfig = supported.into();
    Ok((device, config, sample_format, channels))
}

fn build_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    channels: u16,
    tx: Sender<Vec<i16>>,
) -> Result<cpal::Stream> {
    let ch = channels as usize;
    let err_fn = |e| eprintln!("[agent] mic stream error: {e}");
    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_input_stream(
            config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mono = data
                    .chunks(ch)
                    .map(|f| {
                        let avg = f.iter().copied().sum::<f32>() / ch as f32;
                        (avg.clamp(-1.0, 1.0) * 32767.0) as i16
                    })
                    .collect::<Vec<i16>>();
                let _ = tx.send(mono);
            },
            err_fn,
            None,
        )?,
        cpal::SampleFormat::I16 => device.build_input_stream(
            config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                let mono = data
                    .chunks(ch)
                    .map(|f| (f.iter().map(|&x| x as i32).sum::<i32>() / ch as i32) as i16)
                    .collect::<Vec<i16>>();
                let _ = tx.send(mono);
            },
            err_fn,
            None,
        )?,
        cpal::SampleFormat::U16 => device.build_input_stream(
            config,
            move |data: &[u16], _: &cpal::InputCallbackInfo| {
                let mono = data
                    .chunks(ch)
                    .map(|f| {
                        let avg = f.iter().map(|&x| x as i32 - 32768).sum::<i32>() / ch as i32;
                        avg as i16
                    })
                    .collect::<Vec<i16>>();
                let _ = tx.send(mono);
            },
            err_fn,
            None,
        )?,
        other => return Err(anyhow!("Unsupported microphone sample format: {other:?}")),
    };
    Ok(stream)
}

fn rms(frame: &[i16]) -> f64 {
    if frame.is_empty() {
        return 0.0;
    }
    let sum: f64 = frame.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum / frame.len() as f64).sqrt()
}

/// Lowercase, keep alphanumerics, collapse everything else to single spaces.
fn normalize(s: &str) -> String {
    let mut out = String::new();
    let mut prev_space = false;
    for c in s.to_lowercase().chars() {
        if c.is_alphanumeric() {
            out.push(c);
            prev_space = false;
        } else if !prev_space && !out.is_empty() {
            out.push(' ');
            prev_space = true;
        }
    }
    out.trim().to_string()
}

/// Return the part of `text` after the first case-insensitive match of `phrase`.
fn strip_through(text: &str, phrase: &str) -> String {
    let lower_text = text.to_lowercase();
    let lower_phrase = phrase.to_lowercase();
    match lower_text.find(&lower_phrase) {
        Some(i) => text[(i + lower_phrase.len()).min(text.len())..].to_string(),
        None => text.to_string(),
    }
}

/// Encode interleaved mono PCM i16 into an in-memory WAV file.
pub fn encode_wav(samples: &[i16], sample_rate: u32) -> Result<Vec<u8>> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec)?;
        for &sample in samples {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
    }
    Ok(cursor.into_inner())
}
