import { Router } from "express";
import { hasOpenAiKey } from "../config/env.js";

export const healthRouter = Router();

healthRouter.get("/", (_req, res) => {
  res.json({ status: "ok", aiConfigured: hasOpenAiKey() });
});
