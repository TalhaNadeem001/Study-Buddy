export interface Settings {
  /** Absolute directory paths the app is permitted to write into. */
  allowedDirectories: string[];
  /** Default target relative to an allowed directory, e.g. "Knowledge/AI/Agent Skills". */
  defaultFolder: string;
  /** Default note name (without extension), used when the user doesn't name one. */
  defaultNote: string;
  /** The spoken phrase that activates capture, e.g. "hey write this note". */
  wakePhrase: string;
  /** How aggressively to transform dictation. */
  rewriteStyle: RewriteStyle;
}

export type RewriteStyle = "concise" | "light" | "verbatim";

/** Body of POST /api/notes */
export interface CreateNoteRequest {
  /** Raw dictated speech. */
  transcript: string;
  /** Optional spoken/explicit folder (relative to an allowed directory). */
  folder?: string;
  /** Optional note name (without extension). */
  note?: string;
}

export interface CreateNoteResult {
  /** Absolute path that was written. */
  path: string;
  /** Whether a new file was created or an existing one appended to. */
  action: "created" | "appended";
  /** The cleaned content that was added. */
  content: string;
}
