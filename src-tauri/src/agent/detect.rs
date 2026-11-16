//! Local detection: sherpa-onnx keyword spotting (wake + end phrases) and Silero
//! VAD (speech segments), both on a single 16 kHz stream. No cloud, works offline.

use anyhow::{anyhow, Result};
use sherpa_onnx::{
    KeywordSpotter, KeywordSpotterConfig, LinearResampler, OnlineStream, SileroVadModelConfig,
    VadModelConfig, VoiceActivityDetector,
};
use std::path::Path;

/// KWS labels: the space-free `@label` we attach to each keyword so detections are
/// unambiguous regardless of the user's phrase text.
pub const WAKE_LABEL: &str = "wake";
pub const END_LABEL: &str = "end";

/// Default phrases (used when settings are empty).
pub const WAKE_PHRASE: &str = "Hey Atlas";
pub const END_PHRASE: &str = "Atlas, save note";

const TARGET_RATE: i32 = 16_000;

#[derive(Debug, Clone)]
pub enum DetectEvent {
    Wake,
    End,
    /// A completed speech segment (16 kHz mono f32).
    Segment(Vec<f32>),
}

pub struct Detectors {
    kws: KeywordSpotter,
    kws_stream: OnlineStream,
    vad: VoiceActivityDetector,
    resampler: Option<LinearResampler>,
}

/// A minimal SentencePiece unigram tokenizer (Viterbi over the model's pieces).
/// sherpa's keyword encoder needs BPE-tokenized keywords, so we tokenize the
/// user's phrase ourselves — this matches `spm.encode` exactly for clean English.
pub struct BpeTokenizer {
    vocab: std::collections::HashMap<String, f32>,
    max_piece_chars: usize,
}

impl BpeTokenizer {
    pub fn load(vocab_path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(vocab_path)
            .map_err(|e| anyhow!("Failed to read bpe.vocab: {e}"))?;
        let mut vocab = std::collections::HashMap::new();
        let mut max_piece_chars = 1;
        for line in content.lines() {
            if let Some((piece, score)) = line.split_once('\t') {
                let score: f32 = score.trim().parse().unwrap_or(0.0);
                max_piece_chars = max_piece_chars.max(piece.chars().count());
                vocab.insert(piece.to_string(), score);
            }
        }
        Ok(Self { vocab, max_piece_chars })
    }

    /// Tokenize to space-joined pieces (e.g. "HEY ATLAS" -> "▁HE Y ▁AT LA S").
    pub fn tokenize(&self, text: &str) -> String {
        // SentencePiece: add a leading ▁ and turn spaces into ▁.
        let mut norm = String::from("\u{2581}");
        for c in text.trim().to_uppercase().chars() {
            norm.push(if c == ' ' { '\u{2581}' } else { c });
        }
        let s: Vec<char> = norm.chars().collect();
        let n = s.len();
        const UNK: f32 = -1e6;
        let mut best = vec![f32::NEG_INFINITY; n + 1];
        best[0] = 0.0;
        let mut back = vec![(0usize, String::new()); n + 1];
        for i in 1..=n {
            let lo = i.saturating_sub(self.max_piece_chars);
            for j in lo..i {
                let piece: String = s[j..i].iter().collect();
                let score = match self.vocab.get(&piece) {
                    Some(v) => *v,
                    None if i - j == 1 => UNK,
                    None => continue,
                };
                if best[j] + score > best[i] {
                    best[i] = best[j] + score;
                    back[i] = (j, piece);
                }
            }
        }
        let mut pieces = Vec::new();
        let mut i = n;
        while i > 0 {
            let (j, piece) = back[i].clone();
            pieces.push(piece);
            i = j;
        }
        pieces.reverse();
        pieces.join(" ")
    }
}

/// Build the inline KWS keyword buffer (pre-tokenized) from the user's phrases.
/// `@wake`/`@end` labels are space-free so detection is unambiguous.
fn keywords_buffer(tok: &BpeTokenizer, wake_phrase: &str, stop_phrase: &str) -> String {
    let clean = |p: &str, fallback: &str| {
        let letters: String = p
            .chars()
            .map(|c| if c.is_ascii_alphabetic() { c } else { ' ' })
            .collect();
        if letters.split_whitespace().next().is_some() {
            letters
        } else {
            fallback.to_string()
        }
    };
    let wake = tok.tokenize(&clean(wake_phrase, WAKE_PHRASE));
    let stop = tok.tokenize(&clean(stop_phrase, END_PHRASE));
    // Wake is sensitive (lower #threshold); end is conservative (higher).
    format!("{wake} :1.5 #0.25 @wake\n{stop} :1.5 #0.35 @end\n")
}

impl Detectors {
    pub fn load(
        models_dir: &Path,
        mic_sample_rate: u32,
        wake_phrase: &str,
        stop_phrase: &str,
    ) -> Result<Self> {
        let kws_dir = models_dir.join("kws");
        let path = |f: &str| kws_dir.join(f).to_string_lossy().to_string();

        let mut cfg = KeywordSpotterConfig::default();
        cfg.model_config.transducer.encoder = Some(path("encoder.onnx"));
        cfg.model_config.transducer.decoder = Some(path("decoder.onnx"));
        cfg.model_config.transducer.joiner = Some(path("joiner.onnx"));
        cfg.model_config.tokens = Some(path("tokens.txt"));
        cfg.model_config.provider = Some("cpu".to_string());
        // Tokenize the user's phrases ourselves (sherpa needs BPE-tokenized keywords).
        let tokenizer = BpeTokenizer::load(&kws_dir.join("bpe.vocab"))?;
        cfg.keywords_buf = Some(keywords_buffer(&tokenizer, wake_phrase, stop_phrase));

        let kws = KeywordSpotter::create(&cfg)
            .ok_or_else(|| anyhow!("Failed to load keyword-spotting model (run scripts/fetch-models.sh)"))?;
        let kws_stream = kws.create_stream();

        let vad_cfg = VadModelConfig {
            silero_vad: SileroVadModelConfig {
                model: Some(models_dir.join("silero_vad.onnx").to_string_lossy().to_string()),
                threshold: 0.5,
                min_silence_duration: 0.8,
                min_speech_duration: 0.25,
                window_size: 512,
                max_speech_duration: 20.0,
            },
            sample_rate: TARGET_RATE,
            num_threads: 1,
            provider: Some("cpu".to_string()),
            ..Default::default()
        };
        let vad = VoiceActivityDetector::create(&vad_cfg, 30.0)
            .ok_or_else(|| anyhow!("Failed to load Silero VAD (run scripts/fetch-models.sh)"))?;

        let resampler = if mic_sample_rate as i32 == TARGET_RATE {
            None
        } else {
            Some(
                LinearResampler::create(mic_sample_rate as i32, TARGET_RATE)
                    .ok_or_else(|| anyhow!("Failed to create resampler"))?,
            )
        };

        Ok(Self {
            kws,
            kws_stream,
            vad,
            resampler,
        })
    }

    /// Feed a mic-rate mono f32 chunk; returns detection events in audio order.
    pub fn feed(&mut self, mic: &[f32]) -> Vec<DetectEvent> {
        let r16: Vec<f32> = match &self.resampler {
            Some(rs) => rs.resample(mic, false),
            None => mic.to_vec(),
        };
        let mut events = Vec::new();
        if r16.is_empty() {
            return events;
        }

        // Keyword spotting (wake + end run continuously — never gated by VAD).
        self.kws_stream.accept_waveform(TARGET_RATE, &r16);
        while self.kws.is_ready(&self.kws_stream) {
            self.kws.decode(&self.kws_stream);
        }
        if let Some(result) = self.kws.get_result(&self.kws_stream) {
            let kw = result.keyword;
            if !kw.is_empty() {
                self.kws.reset(&self.kws_stream);
                if kw == WAKE_LABEL {
                    events.push(DetectEvent::Wake);
                } else if kw == END_LABEL {
                    events.push(DetectEvent::End);
                }
            }
        }

        // VAD segmentation.
        self.vad.accept_waveform(&r16);
        while !self.vad.is_empty() {
            if let Some(seg) = self.vad.front() {
                events.push(DetectEvent::Segment(seg.samples().to_vec()));
            }
            self.vad.pop();
        }
        events
    }

    pub fn reset(&mut self) {
        self.kws.reset(&self.kws_stream);
        self.vad.clear();
        if let Some(rs) = &self.resampler {
            rs.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizer_matches_sentencepiece() {
        let vocab = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/models/kws/bpe.vocab");
        if !vocab.exists() {
            return; // models not fetched
        }
        let t = BpeTokenizer::load(&vocab).unwrap();
        assert_eq!(t.tokenize("HEY ATLAS"), "\u{2581}HE Y \u{2581}AT LA S");
        assert_eq!(t.tokenize("hello world"), "\u{2581}HE LL O \u{2581}WORLD");
        assert_eq!(t.tokenize("goodbye now"), "\u{2581}GOOD B Y E \u{2581}NOW");
    }
}
