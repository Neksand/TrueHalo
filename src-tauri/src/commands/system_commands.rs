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
