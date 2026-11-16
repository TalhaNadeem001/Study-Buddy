import { Mic, Sparkles } from "lucide-react";
import type { Capture } from "../types";

/** Shows a capture: what was heard (transcript) vs. what was written (AI note). */
export default function CaptureCard({ capture }: { capture: Capture }) {
  return (
    <div className="rounded-lg border border-slate-200 bg-white p-4">
      <div className="mb-2 flex items-center justify-between">
        <span className="text-xs font-medium text-slate-400">
          {capture.path.split("/").pop()}
        </span>
        <time className="text-xs text-slate-400">
          {new Date(capture.createdAt).toLocaleString()}
        </time>
      </div>

      <div className="flex items-start gap-2 text-sm text-slate-500">
        <Mic className="mt-0.5 h-4 w-4 shrink-0 text-slate-400" />
        <p className="italic">“{capture.transcript}”</p>
      </div>

      <div className="mt-3 flex items-start gap-2 text-sm text-slate-800">
        <Sparkles className="mt-0.5 h-4 w-4 shrink-0 text-indigo-500" />
        <p className="whitespace-pre-wrap">{capture.content}</p>
      </div>
    </div>
  );
}
