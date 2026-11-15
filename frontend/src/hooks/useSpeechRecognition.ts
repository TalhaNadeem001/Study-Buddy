import { useCallback, useEffect, useRef, useState } from "react";

// ---- Minimal Web Speech API typings (not in lib.dom for all setups) ----
interface SpeechRecognitionResultLike {
  0: { transcript: string };
  isFinal: boolean;
}
interface SpeechRecognitionEventLike {
  resultIndex: number;
  results: ArrayLike<SpeechRecognitionResultLike>;
}
interface SpeechRecognitionLike {
  continuous: boolean;
  interimResults: boolean;
  lang: string;
  start: () => void;
  stop: () => void;
  abort: () => void;
  onresult: ((e: SpeechRecognitionEventLike) => void) | null;
  onend: (() => void) | null;
  onerror: ((e: { error: string }) => void) | null;
}
type SpeechRecognitionCtor = new () => SpeechRecognitionLike;

function getRecognitionCtor(): SpeechRecognitionCtor | null {
  const w = window as unknown as {
    SpeechRecognition?: SpeechRecognitionCtor;
    webkitSpeechRecognition?: SpeechRecognitionCtor;
  };
  return w.SpeechRecognition ?? w.webkitSpeechRecognition ?? null;
}

const normalize = (s: string) =>
  s.toLowerCase().replace(/[^\w\s]/g, "").replace(/\s+/g, " ").trim();

/** Return the substring of `text` after the first occurrence of `phrase` (case-insensitive). */
function afterPhrase(text: string, phrase: string): string {
  const idx = text.toLowerCase().indexOf(phrase.toLowerCase());
  return idx === -1 ? "" : text.slice(idx + phrase.length);
}

export type ListenStatus = "idle" | "listening" | "capturing";

interface Options {
  wakePhrase: string;
  onCapture: (text: string) => void;
  /** Phrases that end a capture. */
  stopPhrases?: string[];
  /** How long to wait after speech stops before finalizing a capture (ms). */
  silenceMs?: number;
}

const DEFAULT_STOP_PHRASES = ["end note", "stop note", "thats it", "that's it", "end of note"];

export function useSpeechRecognition({
  wakePhrase,
  onCapture,
  stopPhrases = DEFAULT_STOP_PHRASES,
  silenceMs = 4000,
}: Options) {
  const supported = typeof window !== "undefined" && getRecognitionCtor() !== null;

  const [listening, setListening] = useState(false);
  const [status, setStatus] = useState<ListenStatus>("idle");
  const [interim, setInterim] = useState("");
  const [error, setError] = useState<string | null>(null);

  // Refs so the long-lived recognition handlers always see current values.
  const recognitionRef = useRef<SpeechRecognitionLike | null>(null);
  const enabledRef = useRef(false);
  const capturingRef = useRef(false);
  const bufferRef = useRef("");
  const silenceTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const wakeRef = useRef(wakePhrase);
  const stopRef = useRef(stopPhrases);
  const onCaptureRef = useRef(onCapture);
  useEffect(() => {
    wakeRef.current = wakePhrase;
  }, [wakePhrase]);
  useEffect(() => {
    stopRef.current = stopPhrases;
  }, [stopPhrases]);
  useEffect(() => {
    onCaptureRef.current = onCapture;
  }, [onCapture]);

  const clearSilence = () => {
    if (silenceTimer.current) {
      clearTimeout(silenceTimer.current);
      silenceTimer.current = null;
    }
  };

  const finalizeCapture = useCallback(() => {
    clearSilence();
    capturingRef.current = false;
    const text = bufferRef.current.trim();
    bufferRef.current = "";
    setInterim("");
    if (enabledRef.current) setStatus("listening");
    if (text) onCaptureRef.current(text);
  }, []);

  const resetSilence = useCallback(() => {
    clearSilence();
    silenceTimer.current = setTimeout(finalizeCapture, silenceMs);
  }, [finalizeCapture, silenceMs]);

  const handleFinal = useCallback(
    (raw: string) => {
      const norm = normalize(raw);
      if (!norm) return;

      if (!capturingRef.current) {
        const wake = normalize(wakeRef.current);
        if (wake && norm.includes(wake)) {
          capturingRef.current = true;
          setStatus("capturing");
          const after = afterPhrase(raw, wakeRef.current).trim();
          bufferRef.current = after ? after + " " : "";
          resetSilence();
        }
        return;
      }

      // Capturing: a stop phrase ends the note immediately.
      const stop = stopRef.current.find((p) => norm.includes(normalize(p)));
      if (stop) {
        const idx = raw.toLowerCase().indexOf(stop.toLowerCase());
        if (idx > 0) bufferRef.current += raw.slice(0, idx);
        finalizeCapture();
        return;
      }

      bufferRef.current += raw + " ";
      resetSilence();
    },
    [finalizeCapture, resetSilence],
  );

  const start = useCallback(() => {
    if (!supported || enabledRef.current) return;
    const Ctor = getRecognitionCtor();
    if (!Ctor) return;

    setError(null);
    enabledRef.current = true;
    setListening(true);
    setStatus("listening");

    const rec = new Ctor();
    rec.continuous = true;
    rec.interimResults = true;
    rec.lang = "en-US";

    rec.onresult = (e) => {
      let live = "";
      for (let i = e.resultIndex; i < e.results.length; i++) {
        const res = e.results[i];
        const text = res[0].transcript;
        if (res.isFinal) handleFinal(text);
        else live += text;
      }
      if (live) setInterim(live);
    };

    rec.onerror = (e) => {
      // Transient errors self-recover via onend → restart; fatal ones stop us.
      if (e.error === "not-allowed" || e.error === "service-not-allowed") {
        setError("Microphone permission denied. Allow mic access and try again.");
        enabledRef.current = false;
        setListening(false);
        setStatus("idle");
      }
    };

    rec.onend = () => {
      // Chrome ends the session periodically; restart while still enabled.
      if (enabledRef.current) {
        try {
          rec.start();
        } catch {
          /* already starting — ignore */
        }
      } else {
        setStatus("idle");
      }
    };

    recognitionRef.current = rec;
    try {
      rec.start();
    } catch {
      /* ignore double-start */
    }
  }, [supported, handleFinal]);

  const stop = useCallback(() => {
    enabledRef.current = false;
    capturingRef.current = false;
    clearSilence();
    bufferRef.current = "";
    setInterim("");
    setListening(false);
    setStatus("idle");
    recognitionRef.current?.abort();
    recognitionRef.current = null;
  }, []);

  const toggle = useCallback(() => {
    if (enabledRef.current) stop();
    else start();
  }, [start, stop]);

  // Clean up on unmount.
  useEffect(() => () => stop(), [stop]);

  return { supported, listening, status, interim, error, start, stop, toggle };
}
