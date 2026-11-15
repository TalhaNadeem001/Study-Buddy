import { useCallback, useState } from "react";
import { Mic, MicOff, Send, Loader2, AlertTriangle, CheckCircle2 } from "lucide-react";
import { useSpeechRecognition } from "../hooks/useSpeechRecognition";
import { createNote, toMessage } from "../services/api";
import StatusIndicator from "./StatusIndicator";
import type { HistoryEntry, Settings } from "../types";

interface Props {
  settings: Settings;
  onNoteCreated: (entry: HistoryEntry) => void;
}

export default function VoiceListener({ settings, onNoteCreated }: Props) {
  const [busy, setBusy] = useState(false);
  const [feedback, setFeedback] = useState<string | null>(null);
  const [sendError, setSendError] = useState<string | null>(null);
  const [manual, setManual] = useState("");

  const noAllowedDirs = settings.allowedDirectories.length === 0;

  const handleCapture = useCallback(
    async (transcript: string) => {
      if (!transcript.trim()) return;
      setBusy(true);
      setSendError(null);
      setFeedback(null);
      try {
        const result = await createNote({ transcript });
        const entry: HistoryEntry = {
          ...result,
          id: `${Date.now()}-${Math.round(Math.random() * 1e6)}`,
          transcript,
          at: Date.now(),
        };
        onNoteCreated(entry);
        setFeedback(`${result.action === "created" ? "Created" : "Appended to"} ${result.path.split("/").pop()}`);
      } catch (err) {
        setSendError(toMessage(err));
      } finally {
        setBusy(false);
      }
    },
    [onNoteCreated],
  );

  const { supported, listening, status, interim, error, toggle } = useSpeechRecognition({
    wakePhrase: settings.wakePhrase,
    onCapture: handleCapture,
  });

  return (
    <div className="space-y-5">
      {noAllowedDirs && (
        <div className="flex items-start gap-2 rounded-lg border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800">
          <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
          <span>
            No writable folders configured yet. Add one under <strong>Settings</strong> before
            dictating.
          </span>
        </div>
      )}

      <div className="flex flex-col items-center gap-4 rounded-xl border border-slate-200 bg-white p-8">
        <button
          onClick={toggle}
          disabled={!supported || noAllowedDirs}
          className={`flex h-24 w-24 items-center justify-center rounded-full transition ${
            listening
              ? "bg-indigo-600 text-white shadow-lg shadow-indigo-200"
              : "bg-slate-100 text-slate-700 hover:bg-slate-200"
          } disabled:cursor-not-allowed disabled:opacity-50`}
          title={supported ? (listening ? "Stop listening" : "Start listening") : "Not supported"}
        >
          {listening ? <Mic className="h-10 w-10" /> : <MicOff className="h-10 w-10" />}
        </button>

        <StatusIndicator status={status} />

        <p className="text-center text-sm text-slate-500">
          {supported ? (
            <>
              Say <span className="font-semibold text-slate-700">“{settings.wakePhrase}”</span>, then
              speak your note. Pause (or say “end note”) to finish.
            </>
          ) : (
            "Speech recognition isn't available in this browser — use Chrome, or type a note below."
          )}
        </p>

        {interim && (
          <p className="w-full rounded-md bg-slate-50 px-3 py-2 text-sm italic text-slate-500">
            {interim}
          </p>
        )}

        {busy && (
          <p className="flex items-center gap-2 text-sm text-slate-500">
            <Loader2 className="h-4 w-4 animate-spin" /> Writing note…
          </p>
        )}
      </div>

      {feedback && (
        <div className="flex items-center gap-2 rounded-lg border border-emerald-200 bg-emerald-50 p-3 text-sm text-emerald-800">
          <CheckCircle2 className="h-4 w-4" />
          {feedback}
        </div>
      )}
      {(sendError || error) && (
        <div className="flex items-center gap-2 rounded-lg border border-rose-200 bg-rose-50 p-3 text-sm text-rose-800">
          <AlertTriangle className="h-4 w-4" />
          {sendError ?? error}
        </div>
      )}

      {/* Manual fallback — works without speech support and is handy for testing. */}
      <div className="rounded-xl border border-slate-200 bg-white p-4">
        <label className="mb-2 block text-sm font-medium text-slate-700">
          …or type a note
        </label>
        <textarea
          value={manual}
          onChange={(e) => setManual(e.target.value)}
          rows={3}
          placeholder="Type what you'd otherwise dictate…"
          className="w-full resize-y rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none focus:ring-1 focus:ring-indigo-500"
        />
        <button
          onClick={() => {
            const t = manual.trim();
            if (!t) return;
            setManual("");
            void handleCapture(t);
          }}
          disabled={busy || noAllowedDirs || !manual.trim()}
          className="mt-2 flex items-center gap-1.5 rounded-md bg-indigo-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Send className="h-4 w-4" /> Write note
        </button>
      </div>
    </div>
  );
}
