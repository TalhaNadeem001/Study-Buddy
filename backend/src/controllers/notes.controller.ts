import type { NextFunction, Request, Response } from "express";
import { parseCreateNote } from "../middleware/validate.js";
import { assertAllowed } from "../middleware/pathGuard.js";
import { loadSettings } from "../services/settings.service.js";
import { rewriteNote } from "../services/ai.service.js";
import { resolveTargetPath, readNote, writeNote } from "../services/file.service.js";
import type { CreateNoteResult } from "../types/index.js";

/** POST /api/notes — rewrite dictation and write it to a note. */
export async function createNote(req: Request, res: Response, next: NextFunction): Promise<void> {
  try {
    const { transcript, folder, note } = parseCreateNote(req.body);
    const settings = await loadSettings();

    const targetFolder = folder ?? settings.defaultFolder;
    const targetNote = note ?? settings.defaultNote;

    // Resolve where it goes and enforce the permission boundary before any AI call.
    const { absPath, exists } = await resolveTargetPath(targetFolder, targetNote, settings);
    assertAllowed(absPath, settings);

    const existingContent = exists ? await readNote(absPath) : undefined;
    const content = await rewriteNote({
      transcript,
      style: settings.rewriteStyle,
      existingContent,
    });

    if (!content.trim()) {
      res.status(422).json({ error: "Nothing to write after processing the dictation." });
      return;
    }

    const action = await writeNote(absPath, content, exists);

    const result: CreateNoteResult = { path: absPath, action, content };
    res.status(action === "created" ? 201 : 200).json(result);
  } catch (err) {
    next(err);
  }
}
