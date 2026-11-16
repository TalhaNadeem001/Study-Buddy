import { useCallback, useEffect, useState } from "react";
import { listNotes, onNoteCreated, searchNotes, toMessage } from "../services/ipc";
import type { NoteRecord } from "../types";

/**
 * Notes are now backed by SQLite in the Rust agent (not localStorage).
 * Reloads automatically whenever the agent emits `agent://note-created`.
 */
export function useNotes() {
  const [notes, setNotes] = useState<NoteRecord[]>([]);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    try {
      setNotes(await listNotes());
      setError(null);
    } catch (err) {
      setError(toMessage(err));
    }
  }, []);

  const search = useCallback(
    async (query: string) => {
      try {
        setNotes(query.trim() ? await searchNotes(query) : await listNotes());
        setError(null);
      } catch (err) {
        setError(toMessage(err));
      }
    },
    [],
  );

  useEffect(() => {
    reload();
    const un = onNoteCreated(() => reload());
    return () => {
      un.then((f) => f());
    };
  }, [reload]);

  return { notes, error, reload, search };
}
