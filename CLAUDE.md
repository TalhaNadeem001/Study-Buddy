# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Code search (hard rule)

ALWAYS use the Graphify / codebase-memory MCP tools **first** when searching or exploring this
codebase, before falling back to Grep/Glob/Read:

- `search_graph` — find functions, structs, enums, routes by name/label/qualified-name
- `trace_path` — follow call chains and data flow
- `get_code_snippet` — a symbol's exact source
- `query_graph` — complex graph queries; `get_architecture` — structure overview
- `search_code` — graph-augmented text search

If the repo isn't indexed yet, run `index_repository` first. Grep/Glob/Read are only for
non-code files, configs, and reading a file immediately before editing it.

## Project

Study Buddy AI is a **desktop study assistant**: an always-on background agent that captures notes, organizes them into Markdown files (Obsidian-friendly), and — in later phases — listens for a wake phrase, transcribes speech on-device, and proactively quizzes you with spaced repetition.

It is a **Tauri 2 desktop app**: a React UI shell over a long-running **Rust background agent**, with **SQLite + FTS5** local storage. This is a ground-up rewrite of an earlier web stack (Express backend + browser React); that stack is gone.

> The build is deliberately phased. **Phase 1 (current)** stands up every layer and the boundaries between them, and makes note-taking work end-to-end via a text box. The heavy subsystems (audio, wake word, STT, TTS, spaced repetition, cloud AI) are **scaffolded as stubs** and filled in per phase — see the roadmap at the bottom.

## Architecture

```
Tauri 2 Desktop UI (React)            src/            — can be hidden; agent keeps running
        │  commands (invoke) / events (listen)
        ▼
Rust Background Agent                  src-tauri/src/agent/
  audio · voice · orchestrator · scheduler   (long-running, independent of the window)
        │
        ▼
Local Storage: SQLite + FTS5          src-tauri/src/storage/
Optional Cloud AI (later)             src-tauri/src/ai/   (provider abstraction)
```

### Frontend (`src/`) — Vite + React 18 + TS + Tailwind

- **Routing:** `App.tsx` uses react-router under a `Layout`, with `HomePage` (capture), `NotesPage` (history + search), `SettingsPage`. `main.tsx` uses **`HashRouter`** (not BrowserRouter) because the webview loads over a custom/file protocol in production.
- **IPC is the only transport.** There is no HTTP backend. `services/ipc.ts` wraps Tauri `invoke()` (commands) and `listen()` (events). Never reintroduce axios/`fetch` to a local server.
- **Hooks:** `useSettings` (get/set via IPC), `useNotes` (SQLite-backed list/search, auto-reloads on the `agent://note-created` event), `useAgentStatus` (subscribes to the `agent://status` heartbeat).
- **Vite config:** `base: "./"`, dev server on **:5173** (`strictPort`), Rust `target/` ignored by the watcher. No `/api` proxy.

### Backend (`src-tauri/`) — Tauri 2 + Rust, layered

Strict layering: **commands → agent/orchestrator → services (notes, storage, ai)**. The crate is split into a library (`study_buddy_ai_lib`) with a thin `main.rs` shim, so integration tests (and later mobile targets) can share the code.

- **`lib.rs`** — `run()` builds the Tauri app: registers plugins (notification, autostart), the command handlers, the window-close→hide behavior, the system tray, managed state (`Storage`, `AiState`), and spawns the agent.
- **`commands.rs`** — the IPC surface. Commands return `Result<T, String>` (Tauri serializes the `Err` string to the JS catch). App-defined commands are **not** gated by the capability ACL — only core/plugin commands are (`capabilities/default.json` grants `core:default`).
- **`agent/`** — the long-running agent. `mod.rs::spawn` starts the subsystems and a heartbeat thread emitting `agent://status`.
  - **`orchestrator.rs`** — two write pathways: `process_transcript` (the `create_note` command, settings/Obsidian path, `notes` table) and `process_capture` (voice + the `capture_text` command): rewrite → append the **hardcoded** `~/Documents/StudyBuddy/notes.md` → record a row in `captures` (raw transcript + written note) → mirror the file into `notes` → emit `agent://capture-created` + `agent://note-created`. The allowed-directory guard still gates `process_transcript`.
  - **`audio.rs`** — the wake-phrase listener: `cpal` mic capture → energy-based VAD segments utterances → OpenAI transcription → a `idle→listening→capturing` state machine keyed off the user's `wakePhrase` (and stop phrases like "end note"). Runs on its own OS thread (so `reqwest::blocking` never touches tokio) and emits `agent://voice-state`. NOTE: Porcupine/`pv_*` were the original plan but their entire Rust crate line is **yanked from crates.io**, hence the cpal + OpenAI approach.
  - `voice.rs`, `scheduler.rs` remain Phase-2/4 stubs.
- **`storage/`** — `Storage` owns a single `Mutex<rusqlite::Connection>` held in Tauri state. `schema.rs` creates `notes` + an FTS5 mirror (`notes_fts`, synced by triggers) + a single-row `settings` JSON blob + a `captures` table (transcript + content, for the Voice tab) + a `cards` table reserved for spaced repetition. rusqlite uses `features = ["bundled"]`, which compiles SQLite **with FTS5**.
- **`ai/`** — `rewrite(&settings, ...)` picks OpenAI when a key is set (via `ai::openai::chat_rewrite`), else `local_cleanup` (ported regex filler-word cleanup); it falls back to `local_cleanup` on OpenAI error so a note is never lost. **`ai/openai.rs`** wraps the OpenAI HTTP API: `transcribe` (multipart → `/v1/audio/transcriptions`, default `gpt-transcribe`) and `chat_rewrite` (`/v1/chat/completions`, default `gpt-4o-mini`), with the style prompts ported from the old `ai.service.ts`. Blocking reqwest — callers must run it off the tokio runtime (voice thread, or the thread `capture_text`/`create_note` spawn via `run_blocking`).
- **`notes.rs`** — filesystem layer: `resolve_target_path` matches spoken folder/note names against existing entries **case-insensitively**; notes land inside `allowedDirectories[0]`; `.md` is appended automatically; `write_note` creates or appends with a blank-line separator. Ported from the old `file.service.ts`.
- **`settings.rs`** — the `Settings` struct (serde `camelCase` to match the TS shape exactly across the IPC boundary). `normalized()` trims/dedupes directories.

### The IPC contract (keep both sides in sync by hand)

The Rust `#[derive(Serialize/Deserialize)]` structs use `rename_all = "camelCase"` to mirror `src/types/index.ts`. Changing one **requires** changing the other.

| Command | Args | Returns |
|---|---|---|
| `get_settings` | — | `Settings` |
| `set_settings` | `{ settings }` | `Settings` (normalized) |
| `create_note` | `{ input: CreateNoteInput }` | `CreateNoteResult` (settings-path) |
| `capture_text` | `{ transcript }` | `Capture` (hardcoded file + captures) |
| `list_notes` | — | `NoteRecord[]` |
| `search_notes` | `{ query }` | `NoteRecord[]` (FTS5) |
| `list_captures` | — | `Capture[]` |
| `set_listening` | `{ enabled }` | — (starts/stops the mic listener) |
| `get_listening` | — | `bool` |
| `agent_status` | — | `AgentStatus` |

Events (Rust → UI): `agent://status` (heartbeat), `agent://note-created`, `agent://capture-created` (`Capture`), `agent://voice-state` (`{ state, message }` — idle/listening/capturing/transcribing/writing/error).

## Commands

From the repo root:

```bash
npm install            # JS deps incl. @tauri-apps/cli (first run)
npm run dev            # tauri dev — launches the app (runs vite + the Rust shell)
npm run build          # tauri build — production bundle
npm run build:vite     # frontend-only: tsc typecheck + vite build
```

Rust (from `src-tauri/`):

```bash
cargo build            # compile the app crate
cargo test             # run the headless core tests (tests/core.rs)
cargo clippy           # lint (if installed)
```

**Verification without a GUI:** `cargo test` in `src-tauri/` exercises the storage (SQLite + FTS5 search), AI cleanup, and note path-resolution/writing core headlessly. The full app (tray, window-hide, heartbeat) requires `npm run dev` on a machine with a display.

## Conventions & gotchas

- **Prerequisites:** Rust (rustup, stable) and Xcode Command Line Tools on macOS. Node is pinned to **25** (`.nvmrc`). Tauri CLI is a dev dependency — no global install.
- **No HTTP, no second process.** The app is a single process; the "backend" is Rust in-process. Don't add a server or `localhost` calls.
- **Path guard is the security boundary.** Always resolve + `is_within_allowed` before writing. Preserve the ordering in `process_transcript`.
- **App data** (SQLite db) lives in the OS app-data dir (`app_data_dir()`), not the repo.
- **Settings has no cloud yet:** `aiConfigured` is always `false` in Phase 1; note cleanup is local/on-device.

## Setup

- **Fetch voice models first:** `./scripts/fetch-models.sh` downloads the sherpa-onnx KWS model
  + Silero VAD into `src-tauri/resources/models/` (gitignored, ~14 MB). Required before the voice
  listener works.
- Enter your **OpenAI API key** in Settings (or the `OPENAI_API_KEY` env var) to enable transcription
  + AI rewriting. Voice saves to the hardcoded `~/Documents/StudyBuddy/notes.md`.
- macOS mic permission: `src-tauri/Info.plist` declares `NSMicrophoneUsageDescription`; in
  `tauri dev` you may still need to allow the app under System Settings → Privacy → Microphone.

## Voice listener (`src-tauri/src/agent/`)

Background listener: **one cpal stream** (`capture.rs`, bounded queue + heartbeat/overflow) →
**sherpa-onnx local keyword spotting** (wake "Hey Atlas" / end "Atlas, save note") + **Silero VAD**
segments (`detect.rs`, all offline) → a **deterministic state machine** (`listener.rs::Machine`,
ARMED/RECORDING/… driven only from the capture thread so transitions are serialized) → per-VAD-segment
batch **OpenAI `gpt-transcribe`** on a separate manager thread → assembled transcript → existing
`orchestrator::process_capture` pipeline. Only the **end phrase** finishes a note — pauses never do;
each note has a stable id so a new note can start while the previous finalizes. Phrases are centralized
constants in `detect.rs` (tokenized into `resources/models/kws/keywords.txt`). (Phase B — realtime
`gpt-live-transcribe` + durable recovery — is not built yet.)

## Roadmap (phases)

- **Phase 1 (done):** desktop shell, tray + autostart, SQLite+FTS5, local AI cleanup, text capture, Obsidian-ready file writes.
- **Phase 2 — Voice (done, first pass):** `cpal` mic capture → energy VAD → OpenAI transcription, wake/stop-phrase state machine in `agent/audio.rs`. (Porcupine/Silero/whisper.cpp deferred — the Porcupine Rust crates are yanked; local offline STT is a later swap.)
- **Phase 3 — OpenAI (done, first pass):** `ai/openai.rs` transcription + chat rewrite. Later: command interpretation, quiz generation.
- **Phase 4 — Scheduler + SRS:** `fsrs-rs` over the `cards` table + native notifications ("the fairy").
- **Phase 5 — Optional cloud sync; configurable/multi-file save paths.**
