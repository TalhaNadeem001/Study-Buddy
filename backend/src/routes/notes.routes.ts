import { Router } from "express";
import { createNote } from "../controllers/notes.controller.js";

export const notesRouter = Router();

notesRouter.post("/", createNote);
