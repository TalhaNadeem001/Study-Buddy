# Repository Guidelines

## Project Structure & Module Organization
Study Buddy AI is a Tauri 2 desktop app with React 18, TypeScript, Tailwind, and a Rust background agent. `src/` contains UI components, routed pages, hooks, shared types, and `services/ipc.ts` for Tauri communication. `src-tauri/src/` contains IPC commands, `agent/`, `storage/` (SQLite + FTS5), `ai/`, and note filesystem logic. Rust integration tests live in `src-tauri/tests/core.rs`; desktop assets live in `src-tauri/icons/`. See `CLAUDE.md` for architecture and the phased roadmap; voice and cloud AI remain stubs.

## Build, Test, and Development Commands
Use Node 25 (`.nvmrc`), stable Rust, and platform Tauri prerequisites; macOS requires Xcode Command Line Tools.

- `npm install`: install frontend dependencies and the local Tauri CLI.
- `npm run dev`: launch Vite and the desktop app.
- `npm run dev:vite`: run the frontend server alone; native IPC requires Tauri.
- `npm run build:vite`: type-check TypeScript and build frontend assets.
- `npm run build`: create the production desktop bundle.
- From `src-tauri/`, run `cargo test` for core tests and `cargo clippy` for Rust linting.

## Coding Style & Naming Conventions
Use two-space indentation in TypeScript/TSX and standard Rust formatting (`cargo fmt` from `src-tauri/`). Use PascalCase component filenames, `useName` hooks, camelCase TypeScript identifiers, and snake_case Rust functions. TypeScript enables strict checking and rejects unused locals and parameters. No frontend formatter or linter script is configured. Keep Rust serde camelCase payloads synchronized with `src/types/index.ts`.

## Testing Guidelines
Use Rust's built-in test framework with descriptive snake_case test names. Cover storage round trips, FTS search, AI cleanup, and note path resolution/writes. Run `cargo test` and `npm run build:vite` before submitting changes. No numeric coverage threshold or frontend test runner is configured. Manually verify affected UI flows, tray behavior, and window hiding with `npm run dev`.

## Commit & Pull Request Guidelines
History contains only `Initial commit: Study Buddy AI voice note-taker`; no recurring convention is established. Use concise, action-oriented commit subjects. PRs should explain behavior changes, link relevant issues, report validation, and include screenshots for UI changes.

## Architecture & Agent Guardrails
Keep communication through Tauri IPC and preserve commands → orchestrator → services layering. Resolve paths and enforce `is_within_allowed` before writes. Prefer codebase-memory MCP graph tools for code discovery; use text search for configuration, literals, or insufficient graph results.
