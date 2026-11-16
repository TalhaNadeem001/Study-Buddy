//! Shared audio helpers used by the voice listener (capture, detection, transcription).

use anyhow::Result;

pub struct AudioEngine;

impl AudioEngine {
    pub fn start() {
        eprintln!("[agent] AudioEngine: ready (background voice listener available)");
    }
}

/// Convert mono f32 samples (-1.0..=1.0) to PCM16.
pub fn f32_to_i16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
        .collect()
}

/// Encode interleaved mono PCM16 into an in-memory WAV file.
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
