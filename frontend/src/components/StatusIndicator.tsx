import type { ListenStatus } from "../hooks/useSpeechRecognition";

const MAP: Record<ListenStatus, { label: string; dot: string; text: string }> = {
  idle: { label: "Not listening", dot: "bg-slate-400", text: "text-slate-600" },
  listening: { label: "Listening for wake phrase…", dot: "bg-emerald-500 animate-pulse", text: "text-emerald-700" },
  capturing: { label: "Capturing your note…", dot: "bg-indigo-500 animate-pulse", text: "text-indigo-700" },
};

export default function StatusIndicator({ status }: { status: ListenStatus }) {
  const s = MAP[status];
  return (
    <div className="flex items-center gap-2">
      <span className={`h-2.5 w-2.5 rounded-full ${s.dot}`} />
      <span className={`text-sm font-medium ${s.text}`}>{s.label}</span>
    </div>
  );
}
