import { promises as fs } from "node:fs";
import path from "node:path";
import { SETTINGS_PATH, DEFAULT_SETTINGS } from "../config/index.js";
import type { Settings } from "../types/index.js";

/** Read settings from disk, falling back to defaults (and filling missing keys). */
export async function loadSettings(): Promise<Settings> {
  try {
    const raw = await fs.readFile(SETTINGS_PATH, "utf8");
    const parsed = JSON.parse(raw) as Partial<Settings>;
    return { ...DEFAULT_SETTINGS, ...parsed };
  } catch {
    // No file yet (or unreadable) — return defaults.
    return { ...DEFAULT_SETTINGS };
  }
}

/** Merge a partial update into the stored settings and persist it. */
export async function saveSettings(update: Partial<Settings>): Promise<Settings> {
  const current = await loadSettings();
  const next: Settings = { ...current, ...update };

  // Normalise allowed directories to absolute, de-duplicated paths.
  if (update.allowedDirectories) {
    next.allowedDirectories = Array.from(
      new Set(update.allowedDirectories.map((d) => path.resolve(d.trim())).filter(Boolean)),
    );
  }

  await fs.mkdir(path.dirname(SETTINGS_PATH), { recursive: true });
  await fs.writeFile(SETTINGS_PATH, JSON.stringify(next, null, 2), "utf8");
  return next;
}
