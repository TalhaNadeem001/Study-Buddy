import { useCallback, useState } from "react";
import { Mic, MicOff, Send, Loader2, AlertTriangle, CheckCircle2 } from "lucide-react";
import { captureText, toMessage } from "../services/ipc";
import { useVoice } from "../hooks/useVoice";
import type { Settings, VoicePhase } from "../types";

interface Props {
  settings: Settings;
  onCreated: () => void;
}

const PHASE_LABEL: Record<VoicePhase, string> = {
  idle: "Not listening",
  listening: "Listening for your wake phrase…",
  capturing: "Recording your note…",
  transcribing: "Finishing transcription…",
  writing: "Writing the note…",
  completed: "Saved ✓",
  error: "Voice error",
};

const PHASE_DOT: Record<VoicePhase, string> = {
  idle: "bg-slate-400",
  listening: "bg-emerald-500 animate-pulse",
  capturing: "bg-indigo-500 animate-pulse",
  transcribing: "bg-amber-500 animate-pulse",
  writing: "bg-amber-500 animate-pulse",
  completed: "bg-emerald-500",
  error: "bg-rose-500",
};

export default function Composer({ settings, onCreated }: Props) {
  const { listening, voice, error: voiceError, toggle } = useVoice();
  const [busy, setBusy] = useState(false);
  const [feedback, setFeedback] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [text, setText] = useState("");

  const phase: VoicePhase = voice?.state ?? (listening ? "listening" : "idle");
  const noKey = !settings.aiConfigured;

  const submit = useCallback(async () => {
    const transcript = text.trim();
    if (!transcript) return;
    setBusy(true);
    setError(null);
    setFeedback(null);
    try {
      const capture = await captureText(transcript);
      setText("");
      setFeedback(`Saved to ${capture.path.split("/").pop()}`);
      onCreated();
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setBusy(false);
    }
  }, [text, onCreated]);

  return (
    <div className="space-y-5">
      {noKey && (
        <div className="flex items-start gap-2 rounded-lg border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800">
          <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
          <span>
            No OpenAI key set — add one in <strong>Settings</strong> to enable voice and AI note
            rewriting. (Text notes fall back to basic local cleanup.)
          </span>
        </div>
      )}

      <div className="flex flex-col items-center gap-4 rounded-xl border border-slate-200 bg-white p-8">
        <button
          onClick={toggle}
          className={`flex h-24 w-24 items-center justify-center rounded-full transition ${
            listening
              ? "bg-indigo-600 text-white shadow-lg shadow-indigo-200"
              : "bg-slate-100 text-slate-700 hover:bg-slate-200"
          }`}
          title={listening ? "Stop listening" : "Start listening"}
        >
          {listening ? <Mic className="h-10 w-10" /> : <MicOff className="h-10 w-10" />}
        </button>

        <div className="flex items-center gap-2">
          <span className={`h-2.5 w-2.5 rounded-full ${PHASE_DOT[phase]}`} />
          <span className="text-sm font-medium text-slate-600">
            {voice?.message || PHASE_LABEL[phase]}
          </span>
        </div>

        <p className="text-center text-sm text-slate-500">
          {listening ? (
            <>
              Say <span className="font-semibold text-slate-700">“{settings.wakePhrase}”</span>,
              dictate your note, then say{" "}
              <span className="font-semibold text-slate-700">“{settings.stopPhrase}”</span> to finish.
            </>
          ) : (
            <>Click the mic to start listening, or type a note below.</>
          )}
        </p>
      </div>

      {voiceError && (
        <div className="flex items-center gap-2 rounded-lg border border-rose-200 bg-rose-50 p-3 text-sm text-rose-800">
          <AlertTriangle className="h-4 w-4" /> {voiceError}
        </div>
      )}

      {/* Manual text box — also a quick way to verify your OpenAI key without the mic. */}
      <div className="rounded-xl border border-slate-200 bg-white p-4">
        <label className="mb-2 block text-sm font-medium text-slate-700">…or type a note</label>
        <textarea
          value={text}
          onChange={(e) => setText(e.target.value)}
          rows={3}
          placeholder="Type what you'd otherwise dictate…"
          className="w-full resize-y rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none focus:ring-1 focus:ring-indigo-500"
        />
        <div className="mt-2 flex items-center gap-3">
          <button
            onClick={submit}
            disabled={busy || !text.trim()}
            className="flex items-center gap-1.5 rounded-md bg-indigo-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {busy ? <Loader2 className="h-4 w-4 animate-spin" /> : <Send className="h-4 w-4" />}
            Write note
          </button>
          {feedback && (
            <span className="flex items-center gap-1 text-sm text-emerald-700">
              <CheckCircle2 className="h-4 w-4" /> {feedback}
            </span>
          )}
          {error && (
            <span className="flex items-center gap-1 text-sm text-rose-700">
              <AlertTriangle className="h-4 w-4" /> {error}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}
