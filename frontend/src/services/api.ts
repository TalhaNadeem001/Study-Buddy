import axios from "axios";
import type { CreateNoteResult, Settings } from "../types";

const api = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL || "/api",
});

function toMessage(err: unknown): string {
  if (axios.isAxiosError(err)) {
    return err.response?.data?.error ?? err.message;
  }
  return err instanceof Error ? err.message : "Unknown error";
}

export async function fetchSettings(): Promise<Settings> {
  const { data } = await api.get<Settings>("/settings");
  return data;
}

export async function saveSettings(update: Partial<Settings>): Promise<Settings> {
  const { data } = await api.put<Settings>("/settings", update);
  return data;
}

export interface CreateNoteInput {
  transcript: string;
  folder?: string;
  note?: string;
}

export async function createNote(input: CreateNoteInput): Promise<CreateNoteResult> {
  const { data } = await api.post<CreateNoteResult>("/notes", input);
  return data;
}

export { toMessage };
