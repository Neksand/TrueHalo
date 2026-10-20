//! Screen Recording permission helpers (macOS TCC; Windows has no per-app gate).
//!
//! TrueHalo must *itself* ask macOS for screen capture access. When the system
//! dialog is triggered through a spawned `/usr/sbin/screencapture` process the
//! permission grant is never durably attached to the app, which caused the
//! "granted, but asks again on every screenshot" loop.
//!
//! `CGPreflightScreenCaptureAccess` lets us check the current state without
//! showing anything, and `CGRequestScreenCaptureAccess` shows the system prompt.
//! The prompt is only ever shown once per process, so we guard it.

use anyhow::{Context, Result};
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

/// Set after the first `request_permission` call for this process run.
static PROMPT_SHOWN: AtomicBool = AtomicBool::new(false);

/// Whether the app currently holds the Screen Recording permission.
pub fn has_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        unsafe { CGPreflightScreenCaptureAccess() }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Ask macOS to show the Screen Recording permission prompt.
///
/// Returns `true` when the permission is already (or has just become) available.
/// Because the system only presents the dialog once per process, subsequent
/// calls return `false` without spamming the user, and the UI should instead
/// point at System Settings.
pub fn request_permission() -> bool {
    if has_permission() {
        return true;
    }

    if PROMPT_SHOWN.swap(true, Ordering::SeqCst) {
        return false;
    }

    #[cfg(target_os = "macos")]
    {
        unsafe { CGRequestScreenCaptureAccess() }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Whether the system prompt has already been shown during this app run.
pub fn prompt_already_shown() -> bool {
    PROMPT_SHOWN.load(Ordering::SeqCst)
}

/// Open the exact System Settings pane for Screen Recording.
pub fn open_settings() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        // Works on macOS 13+ including Sequoia/Tahoe, the query is ignored when unknown.
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
            .status()
            .context("Failed to open System Settings")?;
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}
