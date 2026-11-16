import { useEffect, useState } from "react";
import { Plus, Trash2, Save, Loader2, CheckCircle2, AlertTriangle, Sparkles, Mic } from "lucide-react";
import { toMessage } from "../services/ipc";
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
  const [rewriteStyle, setRewriteStyle] = useState<RewriteStyle>(settings.rewriteStyle);
  const [openaiApiKey, setOpenaiApiKey] = useState(settings.openaiApiKey);
  const [openaiModel, setOpenaiModel] = useState(settings.openaiModel);
  const [transcribeModel, setTranscribeModel] = useState(settings.transcribeModel);
  const [wakePhrase, setWakePhrase] = useState(settings.wakePhrase);
  const [stopPhrase, setStopPhrase] = useState(settings.stopPhrase);

  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setDirs(settings.allowedDirectories);
    setDefaultFolder(settings.defaultFolder);
    setDefaultNote(settings.defaultNote);
    setRewriteStyle(settings.rewriteStyle);
    setOpenaiApiKey(settings.openaiApiKey);
    setOpenaiModel(settings.openaiModel);
    setTranscribeModel(settings.transcribeModel);
    setWakePhrase(settings.wakePhrase);
    setStopPhrase(settings.stopPhrase);
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
        rewriteStyle,
        openaiApiKey: openaiApiKey.trim(),
        openaiModel: openaiModel.trim() || "gpt-4o-mini",
        transcribeModel: transcribeModel.trim() || "gpt-transcribe",
        wakePhrase: wakePhrase.trim() || "Hey Atlas",
        stopPhrase: stopPhrase.trim() || "Atlas, save note",
      });
      setSaved(true);
      setTimeout(() => setSaved(false), 2500);
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setSaving(false);
    }
  };

  const field =
    "w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-sm text-slate-900 placeholder:text-slate-500 focus:border-indigo-500 focus:outline-none focus:ring-1 focus:ring-indigo-500";

  return (
    <div className="space-y-6">
      {!settings.aiConfigured && (
        <div className="flex items-start gap-2 rounded-lg border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800">
          <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
          <span>
            No OpenAI key set — voice transcription and AI rewriting are disabled (text notes fall
            back to basic local cleanup). Add your key below to enable them.
          </span>
        </div>
      )}

      {/* OpenAI */}
      <section className="space-y-3 rounded-xl border border-slate-200 bg-white p-4">
        <div className="flex items-center gap-2">
          <Sparkles className="h-4 w-4 text-indigo-500" />
          <h2 className="font-semibold">OpenAI</h2>
        </div>
        <p className="text-sm text-slate-500">
          Powers transcription and note rewriting. Stored locally on this device.
        </p>
        <div>
          <label className="mb-1 block text-sm font-medium text-slate-700">API key</label>
          <input
            type="password"
            value={openaiApiKey}
            onChange={(e) => setOpenaiApiKey(e.target.value)}
            placeholder="sk-…"
            className={field}
            autoComplete="off"
          />
          <p className="mt-1 text-xs text-slate-400">
            Also read from the <code className="rounded bg-slate-100 px-1">OPENAI_API_KEY</code> env
            var when left blank.
          </p>
        </div>
        <div className="grid gap-4 sm:grid-cols-2">
          <div>
            <label className="mb-1 block text-sm font-medium text-slate-700">Chat model</label>
            <input
              value={openaiModel}
              onChange={(e) => setOpenaiModel(e.target.value)}
              placeholder="gpt-4o-mini"
              className={field}
            />
            <p className="mt-1 text-xs text-slate-400">Used to rewrite your dictation.</p>
          </div>
          <div>
            <label className="mb-1 block text-sm font-medium text-slate-700">Transcription model</label>
            <input
              value={transcribeModel}
              onChange={(e) => setTranscribeModel(e.target.value)}
              placeholder="gpt-transcribe"
              className={field}
            />
            <p className="mt-1 text-xs text-slate-400">Speech → text for voice capture.</p>
          </div>
        </div>
      </section>

      {/* Voice */}
      <section className="space-y-3 rounded-xl border border-slate-200 bg-white p-4">
        <div className="flex items-center gap-2">
          <Mic className="h-4 w-4 text-indigo-500" />
          <h2 className="font-semibold">Voice</h2>
        </div>
        <p className="text-sm text-slate-500">
          Phrases are detected on-device (offline) — no recording or training. Changes apply the next
          time you start the listener. Clear, multi-word English phrases work best.
        </p>
        <div className="grid gap-4 sm:grid-cols-2">
          <div>
            <label className="mb-1 block text-sm font-medium text-slate-700">Wake phrase</label>
            <input
              value={wakePhrase}
              onChange={(e) => setWakePhrase(e.target.value)}
              placeholder="Hey Atlas"
              className={field}
            />
            <p className="mt-1 text-xs text-slate-400">Say this to start a note, then dictate.</p>
          </div>
          <div>
            <label className="mb-1 block text-sm font-medium text-slate-700">End phrase</label>
            <input
              value={stopPhrase}
              onChange={(e) => setStopPhrase(e.target.value)}
              placeholder="Atlas, save note"
              className={field}
            />
            <p className="mt-1 text-xs text-slate-400">Say this to finish and save. Pauses never end a note.</p>
          </div>
        </div>
      </section>

      {/* Note folders */}
      <section className="rounded-xl border border-slate-200 bg-white p-4">
        <h2 className="font-semibold">Note folders</h2>
        <p className="mt-1 text-sm text-slate-500">
          Absolute paths the app may write notes into. The first one is the default root — point it
          at your Obsidian vault.
        </p>
        <div className="mt-3 space-y-2">
          {dirs.length === 0 && <p className="text-sm text-slate-400">No folders added yet.</p>}
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
