import { Trash2 } from "lucide-react";
import NoteCard from "../components/NoteCard";
import { useHistory } from "../hooks/useHistory";

export default function NotesPage() {
  const { entries, clear } = useHistory();

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-lg font-semibold">Note history</h1>
        {entries.length > 0 && (
          <button
            onClick={clear}
            className="flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm text-slate-600 hover:bg-rose-50 hover:text-rose-600"
          >
            <Trash2 className="h-4 w-4" /> Clear
          </button>
        )}
      </div>

      {entries.length === 0 ? (
        <p className="py-12 text-center text-sm text-slate-400">
          No notes yet. Dictate one from the Listen tab.
        </p>
      ) : (
        <div className="space-y-3">
          {entries.map((e) => (
            <NoteCard key={e.id} entry={e} />
          ))}
        </div>
      )}
      <p className="text-xs text-slate-400">
        History is stored locally in this browser — it reflects what this device sent, not the full
        contents of your files.
      </p>
    </div>
  );
}
