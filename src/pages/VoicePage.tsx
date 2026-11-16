import CaptureCard from "../components/CaptureCard";
import { useCaptures } from "../hooks/useCaptures";

export default function VoicePage() {
  const { captures } = useCaptures();

  return (
    <div className="space-y-4">
      <h1 className="text-lg font-semibold">Voice notes</h1>
      <p className="text-sm text-slate-500">
        Everything the listener heard and wrote, newest first. Saved to{" "}
        <code className="rounded bg-slate-100 px-1 text-xs">~/Documents/StudyBuddy/notes.md</code>.
      </p>

      {captures.length === 0 ? (
        <p className="py-12 text-center text-sm text-slate-400">
          No captures yet. Start listening on the Listen tab, say your wake phrase, and dictate —
          or just type a note.
        </p>
      ) : (
        <div className="space-y-3">
          {captures.map((c) => (
            <CaptureCard key={c.id} capture={c} />
          ))}
        </div>
      )}
    </div>
  );
}
