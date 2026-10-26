use tauri::command;

/// Open an external URL in the user's default browser.
///
/// The Tauri webview does not implement `window.open` for external URLs, so
/// every link in the UI goes through this command. Only HTTPS is accepted:
/// the app must never be able to launch arbitrary schemes or local files.
#[command]
pub fn open_external(url: String) -> Result<(), String> {
    let trimmed = url.trim();
    if !trimmed.starts_with("https://") {
        return Err("Only https:// links can be opened".to_string());
    }

    open::that(trimmed).map_err(|e| e.to_string())
}

/// Replace all three global shortcuts atomically.
///
/// The old set is restored if the OS refuses one of the new shortcuts, so a
/// failed change can never leave the user without a way to show the overlay.
#[command]
pub fn set_hotkeys(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::app_state::AppState>,
    toggle: String,
    region: String,
    fullscreen: String,
) -> Result<(), String> {
    let next = crate::hotkeys::HotkeyConfig {
        toggle: toggle.trim().to_string(),
        region: region.trim().to_string(),
        fullscreen: fullscreen.trim().to_string(),
    };
    next.validate()?;

    let previous = crate::hotkeys::HotkeyConfig::from_db(&state.db);

    if let Err(err) = crate::hotkeys::register(&app, &next) {
        let _ = crate::hotkeys::register(&app, &previous);
        return Err(err);
    }

    if let Err(err) = next.save(&state.db) {
        let _ = crate::hotkeys::register(&app, &previous);
        return Err(err);
    }

    if let Err(err) = crate::refresh_tray(&app, &next) {
        // The shortcuts are already live; a stale menu label is cosmetic.
        tracing::warn!("Could not refresh tray shortcuts: {}", err);
    }

    Ok(())
}
