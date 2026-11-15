import { isWithinAllowed } from "../services/file.service.js";
import { HttpError } from "./errorHandler.js";
import type { Settings } from "../types/index.js";

/**
 * Throw a 403 if the resolved target isn't inside one of the allowed directories.
 * This is the "permissions" boundary — it prevents writing outside the folders the
 * user explicitly allowed (and blocks path traversal, since the path is already resolved).
 */
export function assertAllowed(absPath: string, settings: Settings): void {
  if (settings.allowedDirectories.length === 0) {
    throw new HttpError(400, "No allowed directories configured. Add one in Settings first.");
  }
  if (!isWithinAllowed(absPath, settings.allowedDirectories)) {
    throw new HttpError(403, `Refusing to write outside allowed directories: ${absPath}`);
  }
}
