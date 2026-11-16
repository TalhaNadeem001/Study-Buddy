import type { AgentStatus } from "../types";

/** Shows the background agent's liveness from its heartbeat events. */
export default function StatusIndicator({ status }: { status: AgentStatus | null }) {
  const online = !!status;
  const capturing = status?.listening === true;
  const dot = capturing
    ? "bg-indigo-500 animate-pulse"
    : online
      ? "bg-emerald-500 animate-pulse"
      : "bg-slate-400";
  const label = online ? status!.detail : "Background agent starting…";
  return (
    <div className="flex items-center gap-2">
      <span className={`h-2.5 w-2.5 rounded-full ${dot}`} />
      <span className="text-sm font-medium text-slate-600">{label}</span>
    </div>
  );
}
