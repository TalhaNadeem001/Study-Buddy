import { useState } from "react";
import { Search, X } from "lucide-react";
import NoteCard from "../components/NoteCard";
import { useNotes } from "../hooks/useNotes";

export default function NotesPage() {
  const { notes, search, reload } = useNotes();
  const [q, setQ] = useState("");

  const runSearch = (value: string) => {
    setQ(value);
    void search(value);
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-3">
        <h1 className="text-lg font-semibold">Notes</h1>
        <div className="relative w-64 max-w-full">
          <Search className="pointer-events-none absolute left-2.5 top-2.5 h-4 w-4 text-slate-400" />
          <input
            value={q}
            onChange={(e) => runSearch(e.target.value)}
            placeholder="Search notes…"
            className="w-full rounded-md border border-slate-300 py-2 pl-8 pr-8 text-sm focus:border-indigo-500 focus:outline-none focus:ring-1 focus:ring-indigo-500"
          />
          {q && (
            <button
              onClick={() => runSearch("")}
              className="absolute right-2 top-2 rounded p-0.5 text-slate-400 hover:text-slate-600"
              title="Clear"
            >
              <X className="h-4 w-4" />
            </button>
          )}
        </div>
      </div>

      {notes.length === 0 ? (
        <p className="py-12 text-center text-sm text-slate-400">
          {q ? "No notes match your search." : "No notes yet. Write one from the Listen tab."}
        </p>
      ) : (
        <div className="space-y-3">
          {notes.map((n) => (
            <NoteCard key={n.id} note={n} />
          ))}
        </div>
      )}
      <p className="text-xs text-slate-400">
        Full-text search is powered by SQLite FTS5 in the local agent.{" "}
        <button onClick={() => void reload()} className="text-indigo-600 hover:underline">
          Refresh
        </button>
      </p>
    </div>
  );
}
