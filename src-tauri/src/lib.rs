pub mod ai;
pub mod app_state;
pub mod auth;
pub mod capture;
pub mod commands;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod overlay;
pub mod storage;

use app_state::AppState;
use commands::{ai_commands, auth_commands, capture_commands, storage_commands, system_commands, window_commands};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use storage::db::DatabaseManager;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

/// Minimum delay between two overlay toggles (guards against duplicated key
/// events / key auto-repeat causing a show-hide-show flip-flop).
const TOGGLE_DEBOUNCE_MS: i64 = 180;

static LAST_TOGGLE_MS: AtomicI64 = AtomicI64::new(0);

// Latest window frame, captured on the main thread from window events.
static GEOM_X: AtomicI32 = AtomicI32::new(i32::MIN);
static GEOM_Y: AtomicI32 = AtomicI32::new(i32::MIN);
static GEOM_W: AtomicI32 = AtomicI32::new(0);
static GEOM_H: AtomicI32 = AtomicI32::new(0);
static GEOM_DIRTY: AtomicBool = AtomicBool::new(false);

/// Opt out of macOS "automatic termination".
///
/// A menu-bar utility whose only window is hidden has no visible windows, and
/// AppKit then asks the process to terminate about 1.7 s later (confirmed via
/// `applicationWillTerminate` in the backtrace). Disabling automatic and sudden
/// termination keeps the overlay alive while it is hidden.
#[cfg(target_os = "macos")]
fn disable_auto_termination() {
    use crate::macos;
    use std::ffi::c_void;
    use std::ptr;

    unsafe {
        let process_info_cls = macos::class("NSProcessInfo");
        let string_cls = macos::class("NSString");
        if process_info_cls.is_null() || string_cls.is_null() {
            return;
        }

        let process_info = macos::send(process_info_cls, "processInfo", ptr::null_mut());
        if process_info.is_null() {
            return;
        }

        macos::send(process_info, "disableSuddenTermination", ptr::null_mut());

        let reason = macos::send(
            string_cls,
            "stringWithUTF8String:",
            c"TrueHalo lives in the menu bar".as_ptr() as *mut c_void,
        );

        macos::send(process_info, "disableAutomaticTermination:", reason);

        tracing::info!("macOS automatic/sudden termination disabled");
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Return the main overlay window, recreating it if the OS destroyed it.
///
/// A destroyed window used to take the whole app down with it (Tauri exits when
/// the last window disappears). Recreating it keeps the overlay usable and the
/// menu bar process alive.
fn ensure_main_window(app: &tauri::AppHandle) -> Option<WebviewWindow> {
    if let Some(window) = app.get_webview_window("main") {
        return Some(window);
    }

    tracing::warn!("Main window is missing — recreating it");

    let builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("TrueHalo")
        .inner_size(1000.0, 720.0)
        .min_inner_size(640.0, 500.0)
        .transparent(true);

    #[cfg(target_os = "macos")]
    let builder = builder
        .hidden_title(true)
        .title_bar_style(tauri::TitleBarStyle::Overlay);

    builder
        .build()
        .map_err(|err| {
            tracing::error!("Could not recreate the main window: {}", err);
            err
        })
        .ok()
}

/// Show and focus the overlay window.
fn show_overlay(app: &tauri::AppHandle) {
    if let Some(window) = ensure_main_window(app) {
        let _ = window.unminimize();
        crate::overlay::show(app);
    }
}

/// Toggle the overlay window visibility.
fn toggle_overlay_window(app: &tauri::AppHandle) {
    // Debounce duplicated shortcut events (key repeat, plugin double-fires).
    let now = now_ms();
    let last = LAST_TOGGLE_MS.load(Ordering::Relaxed);
    if now.saturating_sub(last) < TOGGLE_DEBOUNCE_MS {
        tracing::debug!("Alt+Space ignored (debounce)");
        return;
    }
    LAST_TOGGLE_MS.store(now, Ordering::Relaxed);

    if ensure_main_window(app).is_none() {
        return;
    }

    let visible = crate::overlay::is_visible(app);
    tracing::info!("Alt+Space → visible={}", visible);

    if visible {
        crate::overlay::hide(app);
    } else {
        show_overlay(app);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();

    tracing::info!("Initializing TrueHalo macOS application");

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // Must run before anything can hide the only window.
            #[cfg(target_os = "macos")]
            disable_auto_termination();

            // Setup SQLite database in Application Support
            let app_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let db_path = app_data_dir.join("truehalo.db");
            tracing::info!("Connecting to SQLite database at {:?}", db_path);

            let db = DatabaseManager::new(db_path).expect("Failed to initialize TrueHalo database");

            // Restore persisted window preferences before moving the database.
            let hide_from_dock = db
                .get_preference("hide_from_dock")
                .ok()
                .flatten()
                .map(|v| v == "true")
                .unwrap_or(false);
            let always_on_top = db
                .get_preference("always_on_top")
                .ok()
                .flatten()
                .map(|v| v == "true")
                .unwrap_or(false);
            // Blur (vibrancy) is opt-in: true transparency is the default.
            let vibrancy = db
                .get_preference("vibrancy")
                .ok()
                .flatten()
                .map(|v| v == "true")
                .unwrap_or(false);
            // Screen-recording protection is on by default.
            let exclude_recording = db
                .get_preference("exclude_screen_recording")
                .ok()
                .flatten()
                .map(|v| v == "true")
                .unwrap_or(true);

            restore_window_geometry(app.handle(), &db);

            app.manage(AppState::new(db));

            // Apply the persisted capture protection before the first paint, so
            // the overlay is never visible in a recording, even for a frame.
            // On unsupported Windows builds this is a no-op instead of a black box.
            let _ = window_commands::set_screen_recording_protection(
                app.handle().clone(),
                exclude_recording,
            );

            // Persist the overlay frame whenever the user drags or resizes it.
            spawn_geometry_flusher(app.handle().clone());

            // Menu bar (tray) icon: essential when the app is hidden from the Dock.
            setup_tray(app.handle())?;

            if vibrancy {
                if let Some(window) = app.get_webview_window("main") {
                    apply_vibrancy(&window);
                }
            } else if let Some(window) = app.get_webview_window("main") {
                // Make sure no stale material survives a previous run.
                #[cfg(target_os = "macos")]
                let _ = window_vibrancy::clear_vibrancy(&window);
                #[cfg(windows)]
                let _ = window_vibrancy::clear_acrylic(&window);
                #[cfg(not(any(target_os = "macos", windows)))]
                let _ = &window;
            }

            if hide_from_dock {
                #[cfg(target_os = "macos")]
                let _ = app
                    .handle()
                    .set_activation_policy(tauri::ActivationPolicy::Accessory);
            }

            if always_on_top {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_always_on_top(true);
                }
            }

            // Register Global Shortcuts:
            // 1. Alt+Space (Option+Space on macOS): Toggle TrueHalo Overlay
            let handle_for_toggle = app.handle().clone();
            if let Err(e) =
                app.global_shortcut()
                    .on_shortcut("Alt+Space", move |_app, _shortcut, event| {
                        if event.state == ShortcutState::Pressed {
                            toggle_overlay_window(&handle_for_toggle);
                        }
                    })
            {
                tracing::warn!("Could not register Alt+Space global shortcut: {}", e);
            }

            // 2. Alt+Shift+Digit2 (Option+Shift+2): Quick Region Capture
            let handle_for_cap2 = app.handle().clone();
            if let Err(e) = app.global_shortcut().on_shortcut(
                "Alt+Shift+Digit2",
                move |_app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let _ = handle_for_cap2.emit("hotkey:capture-region", ());
                    }
                },
            ) {
                tracing::warn!("Could not register Alt+Shift+2 global shortcut: {}", e);
            }

            // 3. Alt+Shift+Digit3 (Option+Shift+3): Quick Full Screen Capture
            let handle_for_cap3 = app.handle().clone();
            if let Err(e) = app.global_shortcut().on_shortcut(
                "Alt+Shift+Digit3",
                move |_app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let _ = handle_for_cap3.emit("hotkey:capture-fullscreen", ());
                    }
                },
            ) {
                tracing::warn!("Could not register Alt+Shift+3 global shortcut: {}", e);
            }

            Ok(())
        })
        // Closing the window must keep the overlay alive in the menu bar.
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                tracing::info!("CloseRequested → hiding instead of closing");
                api.prevent_close();
                crate::overlay::hide(window.app_handle());
            }
            tauri::WindowEvent::Moved(position) => {
                GEOM_X.store(position.x, Ordering::Relaxed);
                GEOM_Y.store(position.y, Ordering::Relaxed);
                GEOM_DIRTY.store(true, Ordering::Relaxed);
            }
            tauri::WindowEvent::Resized(size) => {
                GEOM_W.store(size.width as i32, Ordering::Relaxed);
                GEOM_H.store(size.height as i32, Ordering::Relaxed);
                GEOM_DIRTY.store(true, Ordering::Relaxed);
            }
            tauri::WindowEvent::Destroyed => {
                tracing::warn!("Window destroyed");
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            // Auth / Keychain
            auth_commands::save_api_key,
            auth_commands::delete_api_key,
            auth_commands::has_api_key,
            auth_commands::repair_keychain_access,
            // AI
            ai_commands::list_models,
            ai_commands::start_stream,
            ai_commands::cancel_stream,
            // Screen Capture
            capture_commands::capture_screen,
            capture_commands::screen_capture_status,
            capture_commands::request_screen_capture_permission,
            capture_commands::open_screen_capture_settings,
            capture_commands::restart_app,
            // System
            system_commands::open_external,
            // SQLite Storage & Preferences
            storage_commands::list_sessions,
            storage_commands::create_session,
            storage_commands::get_session_messages,
            storage_commands::save_message,
            storage_commands::delete_session,
            storage_commands::clear_all_history,
            storage_commands::set_preference,
            storage_commands::get_preference,
            // Window Management
            window_commands::toggle_overlay,
            window_commands::quick_hide,
            window_commands::show_window,
            window_commands::set_always_on_top,
            window_commands::set_vibrancy,
            window_commands::set_hide_from_dock,
            window_commands::set_screen_recording_protection,
        ])
        .build(tauri::generate_context!())
        .expect("error while building TrueHalo application");

    app.run(|_app_handle, event| match event {
        tauri::RunEvent::ExitRequested { code, api, .. } => {
            if code.is_none() {
                // `None` means "the last window went away", not an explicit quit.
                // Keep the menu-bar app alive; the window is recreated on demand.
                tracing::warn!("Implicit exit requested (no windows) → prevented");
                api.prevent_exit();
            } else {
                tracing::info!("Exit requested with code {:?}", code);
            }
        }
        tauri::RunEvent::Exit => tracing::info!("TrueHalo exiting"),
        _ => {}
    });
}

/// Initialise tracing to both stderr and a rotating log file.
///
/// Launching the `.app` through Finder discards stdout, so a file log is the only
/// way to diagnose issues reported after a normal launch.
fn init_logging() {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct SharedFile(Arc<Mutex<std::fs::File>>);

    impl Write for SharedFile {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            match self.0.lock() {
                Ok(mut file) => file.write(buf),
                Err(_) => Ok(buf.len()),
            }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            match self.0.lock() {
                Ok(mut file) => file.flush(),
                Err(_) => Ok(()),
            }
        }
    }

    let log_file = std::env::var("HOME").ok().and_then(|home| {
        let dir = std::path::PathBuf::from(home).join("Library/Logs/TrueHalo");
        std::fs::create_dir_all(&dir).ok()?;
        let path = dir.join("truehalo.log");

        // Simple rotation: keep the log from growing forever.
        if let Ok(meta) = std::fs::metadata(&path) {
            if meta.len() > 2 * 1024 * 1024 {
                let _ = std::fs::rename(&path, dir.join("truehalo.old.log"));
            }
        }

        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()
    });

    let filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive("truehalo=info".parse().unwrap());

    match log_file {
        Some(file) => {
            let writer = SharedFile(Arc::new(Mutex::new(file)));
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_ansi(false)
                .with_writer(move || writer.clone())
                .init();
        }
        None => {
            tracing_subscriber::fmt().with_env_filter(filter).init();
        }
    }

    // A panic in a webview/event-loop callback would otherwise vanish when the
    // app is launched from Finder.
    std::panic::set_hook(Box::new(|info| {
        tracing::error!("PANIC: {}", info);
    }));
}

/// Write the latest window frame to SQLite from a background thread.
///
/// AppKit window getters must run on the main thread, so the main thread only
/// stores numbers in atomics via `WindowEvent::Moved/Resized`; this thread does
/// the (safe) database work. No AppKit calls happen off the main thread.
fn spawn_geometry_flusher(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(1_200));

        if !GEOM_DIRTY.swap(false, Ordering::Relaxed) {
            continue;
        }

        let (x, y) = (
            GEOM_X.load(Ordering::Relaxed),
            GEOM_Y.load(Ordering::Relaxed),
        );
        let (width, height) = (
            GEOM_W.load(Ordering::Relaxed),
            GEOM_H.load(Ordering::Relaxed),
        );

        if x == i32::MIN || width <= 0 || height <= 0 {
            continue;
        }

        let state = app.state::<AppState>();
        let _ = state.db.set_preference("window_x", &x.to_string());
        let _ = state.db.set_preference("window_y", &y.to_string());
        let _ = state.db.set_preference("window_width", &width.to_string());
        let _ = state
            .db
            .set_preference("window_height", &height.to_string());
    });
}

/// Restore the last known window position and size, clamped to a real monitor.
fn restore_window_geometry(app: &tauri::AppHandle, db: &DatabaseManager) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let read = |key: &str| -> Option<i64> {
        db.get_preference(key)
            .ok()
            .flatten()
            .and_then(|value| value.parse::<i64>().ok())
    };

    if let (Some(width), Some(height)) = (read("window_width"), read("window_height")) {
        if (400..=8000).contains(&width) && (300..=8000).contains(&height) {
            let _ = window.set_size(tauri::PhysicalSize::new(width as u32, height as u32));
        }
    }

    if let (Some(x), Some(y)) = (read("window_x"), read("window_y")) {
        let on_screen = window
            .available_monitors()
            .map(|monitors| {
                monitors.iter().any(|monitor| {
                    let position = monitor.position();
                    let size = monitor.size();
                    let margin = 80_i64;
                    x >= (position.x as i64 - margin)
                        && y >= (position.y as i64 - margin)
                        && x < position.x as i64 + size.width as i64
                        && y < position.y as i64 + size.height as i64
                })
            })
            .unwrap_or(false);

        if on_screen {
            let _ = window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
        } else {
            let _ = window.center();
        }
    }
}

/// Apply the native backdrop material behind the overlay.
fn apply_vibrancy(window: &tauri::WebviewWindow) {
    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};

        if let Err(err) = apply_vibrancy(
            window,
            NSVisualEffectMaterial::HudWindow,
            Some(NSVisualEffectState::Active),
            Some(16.0),
        ) {
            tracing::warn!("Could not apply macOS vibrancy: {}", err);
        }
    }

    #[cfg(windows)]
    {
        if let Err(err) = window_vibrancy::apply_acrylic(window, Some((18, 18, 18, 160))) {
            tracing::warn!("Could not apply Windows acrylic: {}", err);
        }
    }
}

/// Build the tray icon with the quick actions the overlay needs.
fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    let (region_hint, screen_hint) = ("⌥⇧2", "⌥⇧3");
    #[cfg(not(target_os = "macos"))]
    let (region_hint, screen_hint) = ("Alt+Shift+2", "Alt+Shift+3");

    let toggle = MenuItemBuilder::with_id("toggle", "Show / Hide").build(app)?;
    let region =
        MenuItemBuilder::with_id("capture_region", format!("Capture Region  {}", region_hint))
            .build(app)?;
    let fullscreen = MenuItemBuilder::with_id(
        "capture_fullscreen",
        format!("Capture Screen  {}", screen_hint),
    )
    .build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit TrueHalo").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&toggle)
        .separator()
        .item(&region)
        .item(&fullscreen)
        .separator()
        .item(&settings)
        .separator()
        .item(&quit)
        .build()?;

    let mut builder = TrayIconBuilder::with_id("truehalo-tray")
        .menu(&menu)
        .tooltip("TrueHalo")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => toggle_overlay_window(app),
            "capture_region" => {
                let _ = app.emit("hotkey:capture-region", ());
            }
            "capture_fullscreen" => {
                let _ = app.emit("hotkey:capture-fullscreen", ());
            }
            "settings" => {
                show_overlay(app);
                let _ = app.emit("ui:open-settings", ());
            }
            "quit" => app.exit(0),
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}
