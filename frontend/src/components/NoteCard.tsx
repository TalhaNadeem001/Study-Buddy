import { FileText, FilePlus2 } from "lucide-react";
import type { HistoryEntry } from "../types";

export default function NoteCard({ entry }: { entry: HistoryEntry }) {
  const Icon = entry.action === "created" ? FilePlus2 : FileText;
  const fileName = entry.path.split("/").pop() ?? entry.path;
  return (
    <div className="rounded-lg border border-slate-200 bg-white p-4">
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 min-w-0">
          <Icon className="h-4 w-4 shrink-0 text-indigo-600" />
          <span className="truncate font-medium" title={entry.path}>
            {fileName}
          </span>
          <span className="shrink-0 rounded-full bg-slate-100 px-2 py-0.5 text-xs text-slate-600">
            {entry.action}
          </span>
        </div>
        <time className="shrink-0 text-xs text-slate-400">
          {new Date(entry.at).toLocaleTimeString()}
        </time>
      </div>
      <p className="mt-2 whitespace-pre-wrap text-sm text-slate-700">{entry.content}</p>
      <p className="mt-2 truncate text-xs text-slate-400" title={entry.path}>
        {entry.path}
      </p>
    </div>
  );
}
