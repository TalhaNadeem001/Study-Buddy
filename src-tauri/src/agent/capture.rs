//! Microphone capture: ONE cpal input stream, armed while listening.
//!
//! The audio callback does the minimum — downmix to mono f32 and push the chunk
//! into a bounded channel, bumping a heartbeat. All inference/resampling/network
//! happens on the consumer side so a slow consumer can never block capture. A
//! full queue sets an overflow flag (reported as a capture interruption) instead
//! of silently losing audio.

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::Arc;

/// Bounded number of in-flight audio chunks (~chunks of a few ms each).
const QUEUE_CHUNKS: usize = 128;

pub struct Capture {
    /// Kept alive to keep the stream running; dropped to stop capture.
    _stream: cpal::Stream,
    pub rx: Receiver<Vec<f32>>,
    pub sample_rate: u32,
    pub heartbeat: Arc<AtomicU64>,
    pub overflow: Arc<AtomicBool>,
}

/// Open the default input device and start streaming mono f32 chunks.
/// Must be called (and the `Capture` kept) on a single owning thread — cpal's
/// `Stream` is not `Send` on macOS.
pub fn open() -> Result<Capture> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow!("No microphone (input device) found"))?;
    let supported = device
        .default_input_config()
        .map_err(|e| anyhow!("No default input config: {e}"))?;
    let sample_format = supported.sample_format();
    let channels = supported.channels() as usize;
    let sample_rate = supported.sample_rate().0;
    let config: cpal::StreamConfig = supported.into();

    let (tx, rx) = sync_channel::<Vec<f32>>(QUEUE_CHUNKS);
    let heartbeat = Arc::new(AtomicU64::new(0));
    let overflow = Arc::new(AtomicBool::new(false));

    let err_fn = |e| eprintln!("[capture] stream error: {e}");
    let hb = heartbeat.clone();
    let of = overflow.clone();

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mono = downmix(data, channels, |x| x);
                push(&tx, &hb, &of, mono);
            },
            err_fn,
            None,
        )?,
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                let mono = downmix(data, channels, |x| x as f32 / 32768.0);
                push(&tx, &hb, &of, mono);
            },
            err_fn,
            None,
        )?,
        cpal::SampleFormat::U16 => device.build_input_stream(
            &config,
            move |data: &[u16], _: &cpal::InputCallbackInfo| {
                let mono = downmix(data, channels, |x| (x as f32 - 32768.0) / 32768.0);
                push(&tx, &hb, &of, mono);
            },
            err_fn,
            None,
        )?,
        other => return Err(anyhow!("Unsupported microphone sample format: {other:?}")),
    };

    stream
        .play()
        .map_err(|e| anyhow!("Failed to start microphone: {e}"))?;

    Ok(Capture {
        _stream: stream,
        rx,
        sample_rate,
        heartbeat,
        overflow,
    })
}

fn downmix<T: Copy>(data: &[T], channels: usize, to_f32: impl Fn(T) -> f32) -> Vec<f32> {
    if channels <= 1 {
        return data.iter().map(|&x| to_f32(x)).collect();
    }
    data.chunks(channels)
        .map(|frame| frame.iter().map(|&x| to_f32(x)).sum::<f32>() / channels as f32)
        .collect()
}

fn push(tx: &SyncSender<Vec<f32>>, hb: &Arc<AtomicU64>, of: &Arc<AtomicBool>, mono: Vec<f32>) {
    hb.fetch_add(1, Ordering::Relaxed);
    match tx.try_send(mono) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => of.store(true, Ordering::Relaxed),
        Err(TrySendError::Disconnected(_)) => {}
    }
}
