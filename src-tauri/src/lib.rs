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
            // Closing the main window only hides it — the background agent keeps
            // running. Quitting is done explicitly from the tray menu. The overlay
            // is driven entirely by the agent, so leave its events alone.
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    let _ = window.hide();
                    api.prevent_close();
                }
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

            // Auto-start the always-on wake-word listener so the app is listening
            // on launch with no button press. Only when voice is configured — an
            // unkeyed start would just error out immediately.
            let settings = app.state::<Storage>().load_settings().unwrap_or_default();
            if crate::ai::api_key(&settings).is_ok() {
                let voice = app.state::<VoiceControl>();
                agent::listener::start_listening(
                    app.handle().clone(),
                    voice.enabled.clone(),
                    voice.running.clone(),
                );
            }

            // The listening overlay is a display-only indicator: make it
            // click-through so it never steals focus or intercepts the pointer.
            // (The agent shows/hides/positions it via the voice-state pipeline.)
            if let Some(overlay) = app.get_webview_window("overlay") {
                let _ = overlay.set_ignore_cursor_events(true);
                // Float it above everything, including fullscreen apps (macOS).
                #[cfg(target_os = "macos")]
                float_over_fullscreen(&overlay);
            }

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

/// Make the overlay float above everything on macOS — including fullscreen apps.
///
/// A fullscreen app lives in its own Space, so an ordinary always-on-top window
/// stays behind it. Setting the NSWindow collection behavior to include
/// `FullScreenAuxiliary` lets the overlay join the frontmost fullscreen Space as
/// an accessory (`CanJoinAllSpaces` + `Stationary` keep it present across Spaces
/// without animating), and raising the window level orders it above other
/// floating panels. Must run on the main thread — `setup()` satisfies that.
#[cfg(target_os = "macos")]
fn float_over_fullscreen(window: &tauri::WebviewWindow) {
    use objc2::{msg_send, runtime::AnyObject};

    // NSWindowCollectionBehavior bit flags.
    const CAN_JOIN_ALL_SPACES: usize = 1 << 0;
    const STATIONARY: usize = 1 << 4;
    const FULL_SCREEN_AUXILIARY: usize = 1 << 8;
    // Above NSFloatingWindowLevel (3) so it sits over other floating windows.
    const NS_STATUS_WINDOW_LEVEL: isize = 25;

    if let Ok(ptr) = window.ns_window() {
        let ns_window = ptr as *mut AnyObject;
        let behavior = CAN_JOIN_ALL_SPACES | STATIONARY | FULL_SCREEN_AUXILIARY;
        unsafe {
            let _: () = msg_send![ns_window, setCollectionBehavior: behavior];
            let _: () = msg_send![ns_window, setLevel: NS_STATUS_WINDOW_LEVEL];
        }
    }
}
