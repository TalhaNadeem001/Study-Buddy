import { useCallback, useEffect, useState } from "react";
import type { HistoryEntry } from "../types";

const KEY = "study-buddy-history";
const MAX = 50;

function read(): HistoryEntry[] {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? "[]") as HistoryEntry[];
  } catch {
    return [];
  }
}

/** Note history persisted to localStorage, shared across pages. */
export function useHistory() {
  const [entries, setEntries] = useState<HistoryEntry[]>(read);

  // Keep multiple tabs / mounts loosely in sync.
  useEffect(() => {
    const onStorage = (e: StorageEvent) => {
      if (e.key === KEY) setEntries(read());
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  const add = useCallback((entry: HistoryEntry) => {
    setEntries((prev) => {
      const next = [entry, ...prev].slice(0, MAX);
      localStorage.setItem(KEY, JSON.stringify(next));
      return next;
    });
  }, []);

  const clear = useCallback(() => {
    localStorage.removeItem(KEY);
    setEntries([]);
  }, []);

  return { entries, add, clear };
}
