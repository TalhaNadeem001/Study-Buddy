//! Scheduler — reminders and spaced-repetition review prompts.
//! Phase 4: schedule reviews with `fsrs-rs` over the `cards` table and fire
//! native OS notifications even while the window is hidden.

use tauri::AppHandle;

pub struct Scheduler;

impl Scheduler {
    pub fn start(_app: AppHandle) {
        // TODO(phase-4): load due `cards`, schedule with fsrs-rs, and notify via
        // the OS notification plugin on the retained `AppHandle`.
        eprintln!("[agent] Scheduler: stub started (no reminders yet)");
    }
}
