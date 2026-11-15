import path from "node:path";
import { fileURLToPath } from "node:url";
import type { Settings } from "../types/index.js";

const here = path.dirname(fileURLToPath(import.meta.url));

/** backend/data/settings.json — created on first write. */
export const SETTINGS_PATH = path.resolve(here, "../../data/settings.json");

/** File extension used for all notes. */
export const NOTE_EXTENSION = ".md";

export const DEFAULT_SETTINGS: Settings = {
  allowedDirectories: [],
  defaultFolder: "",
  defaultNote: "Inbox",
  wakePhrase: "hey write this note",
  rewriteStyle: "concise",
};
