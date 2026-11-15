import { Link } from "react-router-dom";
import { Loader2 } from "lucide-react";
import VoiceListener from "../components/VoiceListener";
import NoteCard from "../components/NoteCard";
import { useSettings } from "../hooks/useSettings";
import { useHistory } from "../hooks/useHistory";

export default function HomePage() {
  const { settings, loading, error } = useSettings();
  const { entries, add } = useHistory();

  if (loading) {
    return (
      <div className="flex items-center justify-center gap-2 py-16 text-slate-500">
        <Loader2 className="h-5 w-5 animate-spin" /> Loading…
      </div>
    );
  }

  if (error || !settings) {
    return (
      <div className="rounded-lg border border-rose-200 bg-rose-50 p-4 text-sm text-rose-800">
        Couldn't reach the backend: {error ?? "unknown error"}. Make sure it's running on port 3001.
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <VoiceListener settings={settings} onNoteCreated={add} />

      {entries.length > 0 && (
        <section>
          <div className="mb-3 flex items-center justify-between">
            <h2 className="font-semibold">Recent notes</h2>
            <Link to="/notes" className="text-sm text-indigo-600 hover:underline">
              View all
            </Link>
          </div>
          <div className="space-y-3">
            {entries.slice(0, 3).map((e) => (
              <NoteCard key={e.id} entry={e} />
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
