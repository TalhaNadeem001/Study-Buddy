import { promises as fs } from "node:fs";
import path from "node:path";
import { NOTE_EXTENSION } from "../config/index.js";
import type { Settings } from "../types/index.js";

/** True if `target` is the same as, or nested inside, one of the allowed directories. */
export function isWithinAllowed(target: string, allowedDirectories: string[]): boolean {
  const resolved = path.resolve(target);
  return allowedDirectories.some((dir) => {
    const root = path.resolve(dir);
    return resolved === root || resolved.startsWith(root + path.sep);
  });
}

async function listDir(dir: string): Promise<string[]> {
  try {
    return await fs.readdir(dir);
  } catch {
    return [];
  }
}

/** Find an existing entry in `dir` whose name matches `name` case-insensitively. */
async function matchEntry(dir: string, name: string): Promise<string | null> {
  const entries = await listDir(dir);
  const lower = name.toLowerCase();
  return entries.find((e) => e.toLowerCase() === lower) ?? null;
}

export interface ResolvedTarget {
  absPath: string;
  exists: boolean;
}

/**
 * Resolve where a note should be written, matching existing folders/files
 * case-insensitively so spoken lowercase names map onto Title Case paths.
 * The target always lands inside the primary allowed directory.
 */
export async function resolveTargetPath(
  folder: string,
  note: string,
  settings: Settings,
): Promise<ResolvedTarget> {
  const root = settings.allowedDirectories[0];
  if (!root) {
    throw new Error("No allowed directory configured. Add one in Settings first.");
  }

  // Walk folder segments, reusing existing directories by case-insensitive match.
  let current = path.resolve(root);
  const segments = (folder || "").split("/").map((s) => s.trim()).filter(Boolean);
  for (const seg of segments) {
    const match = await matchEntry(current, seg);
    current = path.join(current, match ?? seg);
  }

  // Resolve the note file name (append .md if missing).
  const noteName = note.trim() || "Inbox";
  const fileName = noteName.toLowerCase().endsWith(NOTE_EXTENSION)
    ? noteName
    : `${noteName}${NOTE_EXTENSION}`;
  const existingFile = await matchEntry(current, fileName);
  const absPath = path.join(current, existingFile ?? fileName);

  let exists = false;
  try {
    await fs.access(absPath);
    exists = true;
  } catch {
    exists = false;
  }

  return { absPath, exists };
}

/** Read an existing note's content (empty string if it doesn't exist). */
export async function readNote(absPath: string): Promise<string> {
  try {
    return await fs.readFile(absPath, "utf8");
  } catch {
    return "";
  }
}

export type WriteAction = "created" | "appended";

/** Create the note, or append to it with a blank-line separator. */
export async function writeNote(absPath: string, content: string, exists: boolean): Promise<WriteAction> {
  await fs.mkdir(path.dirname(absPath), { recursive: true });

  if (!exists) {
    await fs.writeFile(absPath, content.endsWith("\n") ? content : content + "\n", "utf8");
    return "created";
  }

  const current = await readNote(absPath);
  const base = current.endsWith("\n") ? current : current + "\n";
  const appended = `${base}\n${content}${content.endsWith("\n") ? "" : "\n"}`;
  await fs.writeFile(absPath, appended, "utf8");
  return "appended";
}
