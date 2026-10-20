use crate::capture::{permission, CaptureOptions, CaptureResponse, ScreenCaptureService};
use serde::Serialize;
use std::time::Duration;
use tauri::{command, AppHandle};

#[derive(Debug, Clone, Serialize)]
pub struct ScreenCaptureStatus {
    /// Screen Recording permission currently granted to TrueHalo.
    pub granted: bool,
    /// The system prompt has already been shown during this app run.
    pub prompt_shown: bool,
    /// True when the user has to grant access and/or restart the app.
    pub needs_user_action: bool,
    /// Platform supports the permission flow (TCC on macOS; always granted on Windows).
    pub supported: bool,
}

fn current_status() -> ScreenCaptureStatus {
    let granted = permission::has_permission();
    let prompt_shown = permission::prompt_already_shown();
    ScreenCaptureStatus {
        granted,
        prompt_shown,
        needs_user_action: !granted,
        supported: cfg!(any(target_os = "macos", windows)),
    }
}

/// Check the Screen Recording permission without showing any system dialog.
#[command]
pub fn screen_capture_status() -> ScreenCaptureStatus {
    current_status()
}

/// Ask the OS for Screen Recording access (macOS TCC; no-op when already granted).
///
/// `CGRequestScreenCaptureAccess` only presents its dialog once per process, so
/// repeated clicks just re-report the current status instead of spamming.
#[command]
pub fn request_screen_capture_permission() -> ScreenCaptureStatus {
    tracing::info!("request_screen_capture_permission invoked");
    permission::request_permission();
    current_status()
}

/// Open System Settings → Privacy & Security → Screen & System Audio Recording.
#[command]
pub fn open_screen_capture_settings() -> Result<(), String> {
    tracing::info!("open_screen_capture_settings invoked");
    permission::open_settings().map_err(|e| e.to_string())
}

/// Relaunch TrueHalo. macOS requires a restart for a freshly granted Screen
/// Recording permission to take effect.
#[command]
pub fn restart_app(app: AppHandle) {
    tracing::warn!("restart_app invoked → the process will restart in 250 ms");
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        app.restart();
    });
}

#[command]
pub async fn capture_screen(
    app: AppHandle,
    options: CaptureOptions,
) -> Result<CaptureResponse, String> {
    tracing::info!(
        "capture_screen: interactive={} compress={} hide_window={}",
        options.interactive,
        options.compress,
        options.hide_window
    );

    // Never trigger the system dialog implicitly from a capture attempt.
    if !permission::has_permission() {
        tracing::warn!("capture_screen: screen recording permission is missing");
        return Ok(CaptureResponse::PermissionRequired);
    }
    tracing::info!("capture_screen: permission granted");

    let was_visible = crate::overlay::is_visible(&app);

    // Hide TrueHalo so the overlay can never end up inside its own screenshot.
    if options.hide_window && was_visible {
        tracing::info!("capture_screen: hiding the overlay");
        crate::overlay::hide(&app);
        // Give the compositor a moment to actually remove the window.
        tokio::time::sleep(Duration::from_millis(220)).await;
    }

    // Run screencapture in a blocking thread so the async runtime is never blocked
    let capture_result =
        tokio::task::spawn_blocking(move || ScreenCaptureService::capture(options))
            .await
            .map_err(|e| e.to_string())?;

    if was_visible {
        crate::overlay::show(&app);
        tracing::info!("capture_screen: window restored");
    }

    tracing::info!(
        "capture_screen: finished with {:?}",
        capture_result.as_ref().map(|r| match r {
            CaptureResponse::Success { .. } => "success",
            CaptureResponse::Cancelled => "cancelled",
            CaptureResponse::PermissionRequired => "permission_required",
        })
    );

    capture_result.map_err(|e| e.to_string())
}
