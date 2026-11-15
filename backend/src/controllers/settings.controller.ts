import type { NextFunction, Request, Response } from "express";
import { loadSettings, saveSettings } from "../services/settings.service.js";
import { hasOpenAiKey } from "../config/env.js";

/** GET /api/settings — current settings plus a couple of runtime flags. */
export async function getSettings(_req: Request, res: Response, next: NextFunction): Promise<void> {
  try {
    const settings = await loadSettings();
    res.json({ ...settings, aiConfigured: hasOpenAiKey() });
  } catch (err) {
    next(err);
  }
}

/** PUT /api/settings — merge a partial update and return the saved result. */
export async function updateSettings(req: Request, res: Response, next: NextFunction): Promise<void> {
  try {
    const saved = await saveSettings(req.body ?? {});
    res.json({ ...saved, aiConfigured: hasOpenAiKey() });
  } catch (err) {
    next(err);
  }
}
