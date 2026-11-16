//! Study Buddy AI — Tauri 2 desktop shell over a long-running Rust background
//! agent. Layers: UI (React) <-> commands/events <-> agent (audio, voice,
//! orchestrator, scheduler) <-> storage (SQLite + FTS5) / optional cloud AI.

pub mod agent;
pub mod ai;
pub mod commands;
pub mod notes;
pub mod settings;
pub mod storage;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;

use crate::commands::VoiceControl;
use crate::storage::Storage;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_settings,
            commands::create_note,
            commands::capture_text,
            commands::list_notes,
            commands::search_notes,
            commands::list_captures,
            commands::set_listening,
            commands::get_listening,
            commands::agent_status,
        ])
        .on_window_event(|window, event| {
            // Closing the window only hides it — the background agent keeps
            // running. Quitting is done explicitly from the tray menu.
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .setup(|app| {
            // Local storage: SQLite + FTS5 in the app data dir.
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("study-buddy.db");
            let storage = Storage::new(&db_path)?;
            app.manage(storage);

            // Shared flags for the wake-word listener thread.
            app.manage(VoiceControl::default());

            // System tray so the app is reachable while the window is hidden.
            setup_tray(app)?;

            // Spawn the long-running background agent (independent of the window).
            agent::spawn(app.handle().clone());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Study Buddy AI");
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show Study Buddy", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide Window", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &hide, &quit])?;

    TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Study Buddy AI")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "hide" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

fn show_main<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
