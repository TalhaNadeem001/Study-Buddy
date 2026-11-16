import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AgentStatus,
  Capture,
  CreateNoteInput,
  CreateNoteResult,
  NoteRecord,
  Settings,
  VoiceState,
} from "../types";

// ---- Commands (frontend -> Rust) --------------------------------------------

export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

export function setSettings(settings: Settings): Promise<Settings> {
  return invoke<Settings>("set_settings", { settings });
}

export function createNote(input: CreateNoteInput): Promise<CreateNoteResult> {
  return invoke<CreateNoteResult>("create_note", { input });
}

/** Capture path used by the manual text box + the voice pipeline. */
export function captureText(transcript: string): Promise<Capture> {
  return invoke<Capture>("capture_text", { transcript });
}

export function listNotes(): Promise<NoteRecord[]> {
  return invoke<NoteRecord[]>("list_notes");
}

export function searchNotes(query: string): Promise<NoteRecord[]> {
  return invoke<NoteRecord[]>("search_notes", { query });
}

export function listCaptures(): Promise<Capture[]> {
  return invoke<Capture[]>("list_captures");
}

export function setListening(enabled: boolean): Promise<void> {
  return invoke("set_listening", { enabled });
}

export function getListening(): Promise<boolean> {
  return invoke<boolean>("get_listening");
}

export function agentStatus(): Promise<AgentStatus> {
  return invoke<AgentStatus>("agent_status");
}

// ---- Events (Rust agent -> frontend) ----------------------------------------

export function onAgentStatus(cb: (s: AgentStatus) => void): Promise<UnlistenFn> {
  return listen<AgentStatus>("agent://status", (e) => cb(e.payload));
}

export function onNoteCreated(cb: (n: CreateNoteResult) => void): Promise<UnlistenFn> {
  return listen<CreateNoteResult>("agent://note-created", (e) => cb(e.payload));
}

export function onVoiceState(cb: (s: VoiceState) => void): Promise<UnlistenFn> {
  return listen<VoiceState>("agent://voice-state", (e) => cb(e.payload));
}

export function onCaptureCreated(cb: (c: Capture) => void): Promise<UnlistenFn> {
  return listen<Capture>("agent://capture-created", (e) => cb(e.payload));
}

// Tauri rejects commands with a plain string (our `Result<_, String>`).
export function toMessage(err: unknown): string {
  if (typeof err === "string") return err;
  return err instanceof Error ? err.message : "Unknown error";
}
