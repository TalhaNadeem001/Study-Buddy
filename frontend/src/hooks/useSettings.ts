import { useCallback, useEffect, useState } from "react";
import { fetchSettings, saveSettings, toMessage } from "../services/api";
import type { Settings } from "../types";

export function useSettings() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setSettings(await fetchSettings());
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  const save = useCallback(async (update: Partial<Settings>) => {
    const saved = await saveSettings(update);
    setSettings(saved);
    return saved;
  }, []);

  return { settings, loading, error, reload, save };
}
