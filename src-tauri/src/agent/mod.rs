//! The long-running background agent. It is independent of the UI window: the
//! window merely observes it through `agent://status` events. In later phases
//! the subsystems below do the real work (audio capture, transcription,
//! orchestration, scheduling); for now they are stubs that announce themselves.

pub mod audio;
pub mod orchestrator;
pub mod scheduler;
pub mod voice;

use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Heartbeat/state the agent pushes to the UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatus {
    pub state: String,
    pub listening: bool,
    pub detail: String,
    pub ticks: u64,
}

const HEARTBEAT: Duration = Duration::from_secs(5);
const DETAIL: &str = "Agent running (Phase 1 — voice pipeline not wired yet)";

/// Start the agent. Brings up the (stubbed) subsystems and a heartbeat loop.
pub fn spawn(app: AppHandle) {
    audio::AudioEngine::start();
    voice::VoicePipeline::start();
    scheduler::Scheduler::start(app.clone());

    // A dedicated OS thread keeps the heartbeat alive regardless of the async
    // runtime state; Phase 2 subsystems can move onto tokio tasks as needed.
    std::thread::spawn(move || {
        let mut ticks: u64 = 0;
        loop {
            let status = AgentStatus {
                state: "idle".to_string(),
                listening: false,
                detail: DETAIL.to_string(),
                ticks,
            };
            let _ = app.emit("agent://status", &status);
            ticks = ticks.wrapping_add(1);
            std::thread::sleep(HEARTBEAT);
        }
    });
}

/// Pull-style snapshot for the `agent_status` command.
pub fn snapshot() -> AgentStatus {
    AgentStatus {
        state: "idle".to_string(),
        listening: false,
        detail: DETAIL.to_string(),
        ticks: 0,
    }
}
