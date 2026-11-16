import { useCallback, useEffect, useState } from "react";
import { listCaptures, onCaptureCreated, toMessage } from "../services/ipc";
import type { Capture } from "../types";

/** Voice/text captures (transcript + written note), newest first. */
export function useCaptures() {
  const [captures, setCaptures] = useState<Capture[]>([]);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    try {
      setCaptures(await listCaptures());
      setError(null);
    } catch (err) {
      setError(toMessage(err));
    }
  }, []);

  useEffect(() => {
    reload();
    const un = onCaptureCreated(() => reload());
    return () => {
      un.then((f) => f());
    };
  }, [reload]);

  return { captures, error, reload };
}
