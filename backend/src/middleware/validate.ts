import { HttpError } from "./errorHandler.js";
import type { CreateNoteRequest } from "../types/index.js";

/** Validate and normalise the POST /api/notes body. */
export function parseCreateNote(body: unknown): CreateNoteRequest {
  if (typeof body !== "object" || body === null) {
    throw new HttpError(400, "Request body must be a JSON object.");
  }
  const b = body as Record<string, unknown>;

  const transcript = typeof b.transcript === "string" ? b.transcript.trim() : "";
  if (!transcript) {
    throw new HttpError(400, "`transcript` is required and must be a non-empty string.");
  }

  const folder = typeof b.folder === "string" ? b.folder.trim() : undefined;
  const note = typeof b.note === "string" ? b.note.trim() : undefined;

  return { transcript, folder, note };
}
