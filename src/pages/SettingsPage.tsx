import { Loader2 } from "lucide-react";
import SettingsForm from "../components/SettingsForm";
import { useSettings } from "../hooks/useSettings";

export default function SettingsPage() {
  const { settings, loading, error, save } = useSettings();

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
    <div className="space-y-4">
      <h1 className="text-lg font-semibold">Settings</h1>
      <SettingsForm settings={settings} onSave={save} />
    </div>
  );
}
