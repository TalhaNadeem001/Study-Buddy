export type RewriteStyle = "concise" | "light" | "verbatim";

export interface Settings {
  allowedDirectories: string[];
  defaultFolder: string;
  defaultNote: string;
  wakePhrase: string;
  rewriteStyle: RewriteStyle;
  /** Runtime flag from the backend: is an OpenAI key configured? */
  aiConfigured?: boolean;
}

export interface CreateNoteResult {
  path: string;
  action: "created" | "appended";
  content: string;
}

/** A locally-tracked entry in the note history list. */
export interface HistoryEntry extends CreateNoteResult {
  id: string;
  transcript: string;
  at: number;
}
