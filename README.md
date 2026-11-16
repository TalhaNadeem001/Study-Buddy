# Study Buddy AI

A desktop study assistant that listens for a wake phrase, transcribes what you say, and
writes it up as clean Markdown notes. It runs in the background from a menu-bar tray, so
you can capture a thought without opening anything.

Built with Tauri 2 (Rust) and React. Transcription and note rewriting use OpenAI.

## Features

- **Wake-phrase listener** — say your phrase (e.g. "hey write this note"), dictate, pause to finish.
- **AI note-taking** — OpenAI transcribes your speech and rewrites it into a clean note.
- **Local storage** — notes are saved to Markdown on disk; SQLite + FTS5 powers history and search.
- **Background agent** — keeps running when the window is closed; reachable from the system tray.
- **Manual capture** — type a note instead of speaking (also handy for testing).

## Tech stack

| Layer | Choice |
| --- | --- |
| Desktop shell | Tauri 2 |
| Backend / agent | Rust |
| Mic capture | cpal |
| Transcription + rewrite | OpenAI (`gpt-transcribe`, chat) |
| Storage | SQLite + FTS5 (rusqlite) |
| UI | React + TypeScript + Tailwind |

## Prerequisites

- [Rust](https://rustup.rs) (stable) — plus Xcode Command Line Tools on macOS (`xcode-select --install`)
- Node 25 (see `.nvmrc`)
- An OpenAI API key (for voice + AI rewriting)

## Getting started

```bash
npm install     # installs JS deps, including the Tauri CLI
npm run dev      # launches the app (Vite dev server + Rust shell)
```

Then open **Settings**, paste your OpenAI API key, and set your wake phrase. Click the mic
on the **Listen** tab to start the listener, say your wake phrase, and dictate. Captures show
up in the **Voice** tab, and the note is appended to `~/Documents/StudyBuddy/notes.md`.

> On macOS you may need to grant microphone access under
> System Settings → Privacy → Microphone the first time.

## Scripts

```bash
npm run dev            # run the app in development
npm run build          # build a production desktop bundle
npm run build:vite     # typecheck + build the frontend only
cd src-tauri && cargo test   # run the Rust core tests
```

## Project structure

```
src/           React UI (pages, components, hooks, IPC client)
src-tauri/     Rust: Tauri shell, background agent, storage, OpenAI client
  src/agent/     wake-phrase listener + note orchestration
  src/ai/        OpenAI transcription + rewrite (local cleanup fallback)
  src/storage/   SQLite schema + queries
```

The UI talks to Rust over Tauri commands and events — there's no HTTP backend.

## Roadmap

- Local/offline transcription (whisper.cpp) as an alternative to OpenAI
- Spaced-repetition review prompts and notifications
- Configurable note destinations (e.g. an Obsidian vault)

## License

MIT
