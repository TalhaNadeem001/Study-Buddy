import { useEffect, useState } from "react";
import { onAgentStatus } from "../services/ipc";
import type { AgentStatus } from "../types";

/** Subscribes to the Rust background agent's heartbeat. */
export function useAgentStatus() {
  const [status, setStatus] = useState<AgentStatus | null>(null);

  useEffect(() => {
    const un = onAgentStatus(setStatus);
    return () => {
      un.then((f) => f());
    };
  }, []);

  return status;
}
