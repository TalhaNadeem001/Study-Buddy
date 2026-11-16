export type RewriteStyle = "concise" | "light" | "verbatim";

export interface Settings {
  /** Absolute dirs the app may write notes into. The first is the default root. */
  allowedDirectories: string[];
  defaultFolder: string;
  defaultNote: string;
  rewriteStyle: RewriteStyle;
  /** Spoken phrase that starts a voice note. */
  wakePhrase: string;
  /** Spoken phrase that finishes a voice note. */
  stopPhrase: string;
  /** OpenAI API key (falls back to the OPENAI_API_KEY env var when blank). */
  openaiApiKey: string;
  /** OpenAI chat model used to rewrite notes. */
  openaiModel: string;
  /** OpenAI transcription model. */
  transcribeModel: string;
  /** Runtime flag: is OpenAI configured? (derived, read-only.) */
  aiConfigured?: boolean;
}

export interface CreateNoteInput {
  transcript: string;
  folder?: string;
  note?: string;
}

export interface CreateNoteResult {
  id: number;
  path: string;
  title: string;
  action: "created" | "appended";
  /** The cleaned content that was written/appended. */
  content: string;
}

/** A note row as stored in SQLite (full file body, for history + search). */
export interface NoteRecord {
  id: number;
  path: string;
  title: string;
  body: string;
  createdAt: number;
  updatedAt: number;
}

/** Heartbeat/state emitted by the Rust background agent. */
export interface AgentStatus {
  state: string;
  listening: boolean;
  detail: string;
  ticks: number;
}

/** Live state of the background voice listener. */
export type VoicePhase =
  | "idle"
  | "listening"
  | "capturing"
  | "transcribing"
  | "writing"
  | "completed"
  | "error";

export interface VoiceState {
  state: VoicePhase;
  message?: string | null;
}

/** A voice/text capture: the raw transcript and the AI-written note. */
export interface Capture {
  id: number;
  transcript: string;
  content: string;
  path: string;
  createdAt: number;
}
