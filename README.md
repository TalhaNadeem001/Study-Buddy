# Study Buddy AI

Voice-activated note-taker. The app listens continuously in your browser; when you say a
custom **wake phrase** (e.g. _"hey, write this note"_), it captures what you say, uses AI
(OpenAI / ChatGPT) to rewrite your rambly speech into a concise note, and writes it to a
file you configured on disk — new note or a continuation of an existing one.

It's the standalone-app version of a voice→Obsidian workflow: in **Settings** you link your
file structure (allowed directories), set the wake phrase, and pick a default target.

## Stack

- **Frontend:** Vite + React 18 + TypeScript + Tailwind + react-router + axios
- **Backend:** Node + Express + TypeScript (routes → controllers → services → middleware)
- **AI:** OpenAI (ChatGPT) via the `openai` SDK
- **Voice:** Web Speech API (`SpeechRecognition`) — works best in Chrome

## Project layout

```
study-buddy-ai/
├── backend/           # Express + TS API
│   └── src/
│       ├── routes/        notes, settings, health
│       ├── controllers/   request handlers
│       ├── services/      ai (OpenAI), file (resolve/append/create), settings
│       ├── middleware/     errorHandler, pathGuard, validate
│       └── config/        env + constants
└── frontend/          # Vite + React + TS app
    └── src/
        ├── pages/         HomePage (listener), SettingsPage, NotesPage
        ├── components/    Layout, VoiceListener, StatusIndicator, NoteCard, SettingsForm
        ├── hooks/         useSpeechRecognition, useSettings
        └── services/      axios API client
```

## Setup

```bash
# 1. Install deps for both apps
npm run install:all

# 2. Configure the backend
cp backend/.env.example backend/.env
#   then set OPENAI_API_KEY in backend/.env

# 3. (optional) Configure the frontend API base URL
cp frontend/.env.example frontend/.env

# 4. Run both apps (backend :3001, frontend :5173)
npm run dev
```

Open the frontend in **Chrome**, go to **Settings**, add an allowed directory (where notes
may be written) and a wake phrase, then return to Home and grant microphone access.

## How it works

1. Browser listens continuously; on hearing the wake phrase it captures the dictation.
2. Frontend `POST /api/notes` with the transcript (and optional target folder/note).
3. Backend asks OpenAI to rewrite the transcript into a concise note.
4. The target file is resolved (case-insensitive) within an **allowed directory**;
   the note is appended if the file exists, or created if new.
5. The result (path, action, content) comes back and shows in your note history.

## Security

- The backend only writes inside directories you explicitly allow in Settings
  (`pathGuard` middleware blocks path traversal and anything outside the allowlist).
- `OPENAI_API_KEY` and `backend/data/settings.json` are git-ignored.

## Known limitations (v1)

- Web Speech API requires an open Chrome tab with mic permission and restarts on silence.
  A true always-on background/menubar listener (Electron) is a future upgrade.
