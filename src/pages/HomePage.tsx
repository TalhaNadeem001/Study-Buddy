import { Link } from "react-router-dom";
import { Loader2 } from "lucide-react";
import Composer from "../components/Composer";
import CaptureCard from "../components/CaptureCard";
import { useSettings } from "../hooks/useSettings";
import { useCaptures } from "../hooks/useCaptures";

export default function HomePage() {
  const { settings, loading, error } = useSettings();
  const { captures, reload } = useCaptures();

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
        Couldn't load settings from the agent: {error ?? "unknown error"}.
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <Composer settings={settings} onCreated={reload} />

      {captures.length > 0 && (
        <section>
          <div className="mb-3 flex items-center justify-between">
            <h2 className="font-semibold">Recent</h2>
            <Link to="/voice" className="text-sm text-indigo-600 hover:underline">
              View all
            </Link>
          </div>
          <div className="space-y-3">
            {captures.slice(0, 3).map((c) => (
              <CaptureCard key={c.id} capture={c} />
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
