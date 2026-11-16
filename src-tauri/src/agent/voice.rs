//! Voice Pipeline — speech-to-text (whisper.cpp) and speech output (OS TTS).
//! Phase 2: feed VAD'd audio into whisper.cpp and push transcripts into the
//! orchestrator; synthesize replies with the native OS TTS.

pub struct VoicePipeline;

impl VoicePipeline {
    pub fn start() {
        // TODO(phase-2): wire whisper.cpp transcription + native OS TTS, and
        // forward transcripts to `orchestrator::process_transcript`.
        eprintln!("[agent] VoicePipeline: stub started (no transcription yet)");
    }
}
