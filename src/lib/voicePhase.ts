import type { VoicePhase } from "../types";

// Shared voice-phase presentation, consumed by the in-app Composer status line
// and the standalone listening Overlay window so both stay in sync.

export const PHASE_LABEL: Record<VoicePhase, string> = {
  idle: "Not listening",
  listening: "Listening for your wake phrase…",
  capturing: "Recording your note…",
  transcribing: "Finishing transcription…",
  writing: "Writing the note…",
  completed: "Saved ✓",
  error: "Voice error",
};

export const PHASE_DOT: Record<VoicePhase, string> = {
  idle: "bg-slate-400",
  listening: "bg-emerald-500 animate-pulse",
  capturing: "bg-indigo-500 animate-pulse",
  transcribing: "bg-amber-500 animate-pulse",
  writing: "bg-amber-500 animate-pulse",
  completed: "bg-emerald-500",
  error: "bg-rose-500",
};
