import OpenAI from "openai";
import { env, hasOpenAiKey } from "../config/env.js";
import type { RewriteStyle } from "../types/index.js";

let client: OpenAI | null = null;
function getClient(): OpenAI {
  if (!client) client = new OpenAI({ apiKey: env.openaiApiKey });
  return client;
}

const STYLE_INSTRUCTIONS: Record<RewriteStyle, string> = {
  concise:
    "Rewrite the dictation into tight, clear notes — do not transcribe it. Rephrase freely, " +
    "cut repetition, filler, and tangents, and restructure so the point lands fast. You may add " +
    "a short clarifying line when it genuinely helps. Stay true to the speaker's intent and key " +
    "points; never invent claims they'd disagree with.",
  light:
    "Lightly clean up the dictation: keep the speaker's words and meaning, remove filler " +
    "(\"um\", \"like\", false starts), fix grammar and punctuation, and break it into readable " +
    "paragraphs or bullets. Do not summarize or reorganize.",
  verbatim:
    "Write down what was said almost exactly, with only minimal cleanup (capitalization and " +
    "punctuation). Do not rephrase or restructure.",
};

const FORMAT_RULES =
  "Output only the note body in Markdown — no title, no preamble, no code fences, no YAML " +
  "frontmatter. Use '-' for bullet lists and '####' for a heading only when the note clearly " +
  "has distinct parts. When continuing an existing note, match its terminology and voice, don't " +
  "repeat points already present, and phrase the new content so it fits coherently.";

interface RewriteArgs {
  transcript: string;
  style: RewriteStyle;
  /** Existing note content, when appending — used as context. */
  existingContent?: string;
}

/**
 * Turn raw dictation into a clean note using OpenAI (ChatGPT).
 * Falls back to a trivial local cleanup when no API key is configured,
 * so the app still works end-to-end without a key.
 */
export async function rewriteNote({ transcript, style, existingContent }: RewriteArgs): Promise<string> {
  const text = transcript.trim();
  if (!text) return "";

  if (!hasOpenAiKey()) {
    return localCleanup(text);
  }

  const systemPrompt = `You turn a person's spoken dictation into a note for their personal knowledge base.\n\n${STYLE_INSTRUCTIONS[style]}\n\n${FORMAT_RULES}`;

  const userPrompt = existingContent
    ? `This note already contains the following content:\n"""\n${existingContent}\n"""\n\nThe speaker just dictated this to add to it:\n"""\n${text}\n"""\n\nWrite only the new content to append.`
    : `The speaker dictated this for a new note:\n"""\n${text}\n"""`;

  try {
    const completion = await getClient().chat.completions.create({
      model: env.openaiModel,
      messages: [
        { role: "system", content: systemPrompt },
        { role: "user", content: userPrompt },
      ],
    });
    const out = completion.choices[0]?.message?.content?.trim();
    return out || localCleanup(text);
  } catch (err) {
    // Surface a clear, actionable error to the controller.
    const message = err instanceof Error ? err.message : String(err);
    throw new Error(`OpenAI rewrite failed: ${message}`);
  }
}

/** Minimal offline cleanup: strip filler words, collapse whitespace, capitalize. */
function localCleanup(text: string): string {
  let cleaned = text
    .replace(/\b(um+|uh+|like|you know|i mean|sort of|kind of)\b/gi, "")
    .replace(/\s+/g, " ")
    .replace(/\s+([,.!?])/g, "$1")
    .trim();
  if (cleaned) cleaned = cleaned.charAt(0).toUpperCase() + cleaned.slice(1);
  return cleaned;
}
