import { useEffect, useState } from "react";
import { AudioLines } from "lucide-react";
import { onVoiceState } from "../services/ipc";
import { PHASE_LABEL, PHASE_DOT } from "../lib/voicePhase";
import type { VoicePhase, VoiceState } from "../types";

/**
 * Standalone listening indicator rendered in its own frameless, transparent,
 * always-on-top Tauri window (label "overlay"). The Rust agent owns this
 * window's visibility/position (see agent/listener.rs `update_overlay`); this
 * component only paints whatever voice-state it receives. Display-only — the
 * window is click-through, so there are no interactive controls here.
 */
export default function Overlay() {
  const [voice, setVoice] = useState<VoiceState | null>(null);

  useEffect(() => {
    // Each Tauri window is an isolated webview with its own document, so making
    // the body transparent here does NOT affect the main window's background.
    const prevBg = document.body.style.background;
    document.body.style.background = "transparent";

    const un = onVoiceState(setVoice);
    return () => {
      document.body.style.background = prevBg;
      un.then((f) => f());
    };
  }, []);

  const phase: VoicePhase = voice?.state ?? "listening";

  return (
    <div className="pointer-events-none flex h-screen w-screen items-center justify-center p-2">
      <div className="flex items-center gap-2.5 rounded-xl border border-slate-200 bg-white px-3.5 py-2.5 shadow-lg">
        <AudioLines className="h-4 w-4 shrink-0 text-indigo-600" />
        <span className={`h-2.5 w-2.5 shrink-0 rounded-full ${PHASE_DOT[phase]}`} />
        <span className="truncate text-sm font-medium text-slate-700">
          {voice?.message || PHASE_LABEL[phase]}
        </span>
      </div>
    </div>
  );
}
