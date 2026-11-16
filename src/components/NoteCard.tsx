import { FileText } from "lucide-react";
import type { NoteRecord } from "../types";

export default function NoteCard({ note }: { note: NoteRecord }) {
  const fileName = note.path.split("/").pop() ?? note.title;
  return (
    <div className="rounded-lg border border-slate-200 bg-white p-4">
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 min-w-0">
          <FileText className="h-4 w-4 shrink-0 text-indigo-600" />
          <span className="truncate font-medium" title={note.path}>
            {note.title || fileName}
          </span>
        </div>
        <time className="shrink-0 text-xs text-slate-400">
          {new Date(note.updatedAt).toLocaleString()}
        </time>
      </div>
      <p className="mt-2 line-clamp-6 whitespace-pre-wrap text-sm text-slate-700">{note.body}</p>
      <p className="mt-2 truncate text-xs text-slate-400" title={note.path}>
        {note.path}
      </p>
    </div>
  );
}
