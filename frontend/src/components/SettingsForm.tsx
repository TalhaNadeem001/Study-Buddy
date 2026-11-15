import { useEffect, useState } from "react";
import { Plus, Trash2, Save, Loader2, CheckCircle2, AlertTriangle } from "lucide-react";
import { toMessage } from "../services/api";
import type { RewriteStyle, Settings } from "../types";

interface Props {
  settings: Settings;
  onSave: (update: Partial<Settings>) => Promise<Settings>;
}

const STYLES: { value: RewriteStyle; label: string; help: string }[] = [
  { value: "concise", label: "Concise rewrite", help: "Tighten and rephrase into clean notes." },
  { value: "light", label: "Light cleanup", help: "Keep your words; remove filler, fix grammar." },
  { value: "verbatim", label: "Verbatim", help: "Near word-for-word, minimal edits." },
];

export default function SettingsForm({ settings, onSave }: Props) {
  const [dirs, setDirs] = useState<string[]>(settings.allowedDirectories);
  const [newDir, setNewDir] = useState("");
  const [defaultFolder, setDefaultFolder] = useState(settings.defaultFolder);
  const [defaultNote, setDefaultNote] = useState(settings.defaultNote);
  const [wakePhrase, setWakePhrase] = useState(settings.wakePhrase);
  const [rewriteStyle, setRewriteStyle] = useState<RewriteStyle>(settings.rewriteStyle);

  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setDirs(settings.allowedDirectories);
    setDefaultFolder(settings.defaultFolder);
    setDefaultNote(settings.defaultNote);
    setWakePhrase(settings.wakePhrase);
    setRewriteStyle(settings.rewriteStyle);
  }, [settings]);

  const addDir = () => {
    const d = newDir.trim();
    if (d && !dirs.includes(d)) setDirs([...dirs, d]);
    setNewDir("");
  };

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    setSaved(false);
    try {
      await onSave({
        allowedDirectories: dirs,
        defaultFolder: defaultFolder.trim(),
        defaultNote: defaultNote.trim() || "Inbox",
        wakePhrase: wakePhrase.trim() || "hey write this note",
        rewriteStyle,
      });
      setSaved(true);
      setTimeout(() => setSaved(false), 2500);
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setSaving(false);
    }
  };

  const field = "w-full rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none focus:ring-1 focus:ring-indigo-500";

  return (
    <div className="space-y-6">
      {!settings.aiConfigured && (
        <div className="flex items-start gap-2 rounded-lg border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800">
          <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
          <span>
            No OpenAI key detected on the backend — notes fall back to basic local cleanup. Set{" "}
            <code className="rounded bg-amber-100 px-1">OPENAI_API_KEY</code> in{" "}
            <code className="rounded bg-amber-100 px-1">backend/.env</code> for AI rewriting.
          </span>
        </div>
      )}

      {/* Allowed directories */}
      <section className="rounded-xl border border-slate-200 bg-white p-4">
        <h2 className="font-semibold">Allowed folders</h2>
        <p className="mt-1 text-sm text-slate-500">
          Absolute paths the app may write notes into. The first one is the default root.
        </p>
        <div className="mt-3 space-y-2">
          {dirs.length === 0 && (
            <p className="text-sm text-slate-400">No folders added yet.</p>
          )}
          {dirs.map((d) => (
            <div key={d} className="flex items-center gap-2">
              <code className="flex-1 truncate rounded bg-slate-50 px-2 py-1.5 text-xs" title={d}>
                {d}
              </code>
              <button
                onClick={() => setDirs(dirs.filter((x) => x !== d))}
                className="rounded-md p-1.5 text-slate-400 hover:bg-rose-50 hover:text-rose-600"
                title="Remove"
              >
                <Trash2 className="h-4 w-4" />
              </button>
            </div>
          ))}
        </div>
        <div className="mt-3 flex gap-2">
          <input
            value={newDir}
            onChange={(e) => setNewDir(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && addDir()}
            placeholder="/Users/you/Documents/Obsidian/Vault"
            className={field}
          />
          <button
            onClick={addDir}
            className="flex items-center gap-1 rounded-md bg-slate-100 px-3 text-sm font-medium text-slate-700 hover:bg-slate-200"
          >
            <Plus className="h-4 w-4" /> Add
          </button>
        </div>
      </section>

      {/* Defaults */}
      <section className="grid gap-4 rounded-xl border border-slate-200 bg-white p-4 sm:grid-cols-2">
        <div>
          <label className="mb-1 block text-sm font-medium text-slate-700">Default folder</label>
          <input
            value={defaultFolder}
            onChange={(e) => setDefaultFolder(e.target.value)}
            placeholder="Knowledge/AI/Agent Skills"
            className={field}
          />
          <p className="mt-1 text-xs text-slate-400">Relative to the default root.</p>
        </div>
        <div>
          <label className="mb-1 block text-sm font-medium text-slate-700">Default note</label>
          <input
            value={defaultNote}
            onChange={(e) => setDefaultNote(e.target.value)}
            placeholder="Inbox"
            className={field}
          />
        </div>
        <div>
          <label className="mb-1 block text-sm font-medium text-slate-700">Wake phrase</label>
          <input
            value={wakePhrase}
            onChange={(e) => setWakePhrase(e.target.value)}
            placeholder="hey write this note"
            className={field}
          />
        </div>
        <div>
          <label className="mb-1 block text-sm font-medium text-slate-700">Rewrite style</label>
          <select
            value={rewriteStyle}
            onChange={(e) => setRewriteStyle(e.target.value as RewriteStyle)}
            className={field}
          >
            {STYLES.map((s) => (
              <option key={s.value} value={s.value}>
                {s.label}
              </option>
            ))}
          </select>
          <p className="mt-1 text-xs text-slate-400">
            {STYLES.find((s) => s.value === rewriteStyle)?.help}
          </p>
        </div>
      </section>

      <div className="flex items-center gap-3">
        <button
          onClick={handleSave}
          disabled={saving}
          className="flex items-center gap-1.5 rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-700 disabled:opacity-50"
        >
          {saving ? <Loader2 className="h-4 w-4 animate-spin" /> : <Save className="h-4 w-4" />}
          Save settings
        </button>
        {saved && (
          <span className="flex items-center gap-1 text-sm text-emerald-700">
            <CheckCircle2 className="h-4 w-4" /> Saved
          </span>
        )}
        {error && (
          <span className="flex items-center gap-1 text-sm text-rose-700">
            <AlertTriangle className="h-4 w-4" /> {error}
          </span>
        )}
      </div>
    </div>
  );
}
