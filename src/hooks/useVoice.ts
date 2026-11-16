import { useCallback, useEffect, useState } from "react";
import { getListening, onVoiceState, setListening, toMessage } from "../services/ipc";
import type { VoiceState } from "../types";

/** Controls the wake-phrase listener and tracks its live state. */
export function useVoice() {
  const [listening, setListeningState] = useState(false);
  const [voice, setVoice] = useState<VoiceState | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getListening().then(setListeningState).catch(() => {});
    const un = onVoiceState((s) => {
      setVoice(s);
      if (s.state === "error") {
        setError(s.message ?? "Voice error");
        setListeningState(false);
      }
      if (s.state === "idle") setListeningState(false);
    });
    return () => {
      un.then((f) => f());
    };
  }, []);

  const toggle = useCallback(async () => {
    const next = !listening;
    setError(null);
    try {
      await setListening(next);
      setListeningState(next);
      if (!next) setVoice({ state: "idle" });
    } catch (err) {
      setError(toMessage(err));
    }
  }, [listening]);

  return { listening, voice, error, toggle };
}
