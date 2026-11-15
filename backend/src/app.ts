import express from "express";
import cors from "cors";
import { env } from "./config/env.js";
import { healthRouter } from "./routes/health.routes.js";
import { notesRouter } from "./routes/notes.routes.js";
import { settingsRouter } from "./routes/settings.routes.js";
import { errorHandler } from "./middleware/errorHandler.js";

export function createApp() {
  const app = express();

  app.use(cors({ origin: env.corsOrigin.split(",").map((o) => o.trim()) }));
  app.use(express.json({ limit: "1mb" }));

  app.use("/api/health", healthRouter);
  app.use("/api/notes", notesRouter);
  app.use("/api/settings", settingsRouter);

  // 404 for unmatched API routes.
  app.use((_req, res) => res.status(404).json({ error: "Not found" }));

  app.use(errorHandler);
  return app;
}
