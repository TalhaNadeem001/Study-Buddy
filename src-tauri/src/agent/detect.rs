//! Local detection: sherpa-onnx keyword spotting (wake + end phrases) on a 16 kHz
//! stream. No cloud, works offline. Returns the resampled audio so the listener
//! can accumulate the whole note.

use anyhow::{anyhow, Result};
use sherpa_onnx::{
    KeywordSpotter, KeywordSpotterConfig, LinearResampler, OnlineStream, VadModelConfig,
    VoiceActivityDetector,
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

/// Pre-roll kept in front of the KWS so the onset of the wake phrase isn't clipped
/// by Silero's speech-onset latency: ~500 ms of 16 kHz mono.
const PREROLL_SAMPLES: usize = TARGET_RATE as usize / 2;
/// Grace period after speech stops before we gate the KWS off, measured in fed
/// chunks. Keeps brief inter-word pauses from thrashing the gate. The device
/// delivers chunks every few ms, so ~1 s is comfortably covered by 64 chunks.
const HANGOVER_CHUNKS: u32 = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectEvent {
    Wake,
    End,
}

pub struct Detectors {
    kws: KeywordSpotter,
    kws_stream: OnlineStream,
    resampler: Option<LinearResampler>,
    /// Cheap "is anyone talking?" gate in front of the expensive zipformer KWS.
    vad: VoiceActivityDetector,
    /// Rolling buffer of the most recent 16 kHz samples, fed into the KWS on a
    /// rising speech edge so the wake phrase's start isn't lost.
    preroll: Vec<f32>,
    /// Whether the KWS is currently being fed (speech detected / within hangover).
    active: bool,
    /// Chunks of grace left before gating the KWS off after speech stops.
    hangover: u32,
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

        // Silero VAD: a tiny, cheap speech detector that runs continuously so the
        // heavy zipformer KWS only wakes when someone is actually talking.
        let mut vad_cfg = VadModelConfig::default();
        vad_cfg.silero_vad.model = Some(
            models_dir
                .join("silero_vad.onnx")
                .to_string_lossy()
                .to_string(),
        );
        vad_cfg.silero_vad.threshold = 0.5;
        vad_cfg.silero_vad.min_silence_duration = 0.1;
        vad_cfg.silero_vad.min_speech_duration = 0.05;
        vad_cfg.silero_vad.window_size = 512; // Silero window at 16 kHz
        vad_cfg.sample_rate = TARGET_RATE;
        vad_cfg.num_threads = 1;
        vad_cfg.provider = Some("cpu".to_string());
        let vad = VoiceActivityDetector::create(&vad_cfg, 5.0)
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
            resampler,
            vad,
            preroll: Vec::with_capacity(PREROLL_SAMPLES),
            active: false,
            hangover: 0,
        })
    }

    /// Feed a mic-rate mono f32 chunk; returns the resampled 16 kHz samples (for the
    /// listener to accumulate) and any wake/end events.
    pub fn feed(&mut self, mic: &[f32]) -> (Vec<f32>, Vec<DetectEvent>) {
        let r16: Vec<f32> = match &self.resampler {
            Some(rs) => rs.resample(mic, false),
            None => mic.to_vec(),
        };
        let mut events = Vec::new();
        if r16.is_empty() {
            return (r16, events);
        }

        // Keep the most recent ~500 ms around so a rising speech edge can seed the
        // KWS with the wake phrase's onset (Silero reports speech a beat late).
        self.preroll.extend_from_slice(&r16);
        if self.preroll.len() > PREROLL_SAMPLES {
            let excess = self.preroll.len() - PREROLL_SAMPLES;
            self.preroll.drain(..excess);
        }

        // Cheap gate: run the heavy KWS only when Silero hears speech (plus a short
        // hangover). We only use `detected()`, so drain the segment queue each time
        // to keep VAD memory bounded.
        self.vad.accept_waveform(&r16);
        let speaking = self.vad.detected();
        self.vad.clear();

        if speaking {
            if !self.active {
                // Rising edge: seed the KWS with the pre-roll so the wake onset
                // isn't clipped, then fall through to feed this chunk too.
                self.active = true;
                let preroll = std::mem::take(&mut self.preroll);
                self.kws_stream.accept_waveform(TARGET_RATE, &preroll);
            }
            self.hangover = HANGOVER_CHUNKS;
        } else if self.active {
            if self.hangover > 0 {
                self.hangover -= 1;
            } else {
                // Falling edge: gate off and reset so the next utterance starts
                // from a clean streaming state.
                self.active = false;
                self.kws.reset(&self.kws_stream);
            }
        }

        if !self.active {
            // Silence: skip the expensive encoder entirely.
            return (r16, events);
        }

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

        (r16, events)
    }

    pub fn reset(&mut self) {
        self.kws.reset(&self.kws_stream);
        self.vad.reset();
        self.preroll.clear();
        self.active = false;
        self.hangover = 0;
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
