import { useCallback, useEffect, useState } from "react";
import { getSettings, setSettings, toMessage } from "../services/ipc";
import type { Settings } from "../types";

export function useSettings() {
  const [settings, setSettingsState] = useState<Settings | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setSettingsState(await getSettings());
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  // Accepts a partial update, merges it over the current settings, and persists
  // the full object (the Rust `set_settings` command stores the whole record).
  const save = useCallback(
    async (update: Partial<Settings>) => {
      const merged = { ...(settings ?? ({} as Settings)), ...update } as Settings;
      const saved = await setSettings(merged);
      setSettingsState(saved);
      return saved;
    },
    [settings],
  );

  return { settings, loading, error, reload, save };
}
