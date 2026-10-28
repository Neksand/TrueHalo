//! Show/hide the overlay window safely.
//!
//! Two macOS pitfalls are handled here:
//!
//! * `NSWindow.orderOut` (what `window.hide()` does) makes AppKit terminate the
//!   process about 1.7 s later when it is the last window — verified through
//!   `-[NSApplication terminate:]` / `applicationWillTerminate`. Hiding the
//!   **application** instead keeps the window ordered in, so the process stays
//!   alive while the overlay disappears from the screen (and from screenshots).
//! * AppKit must only be used from the main thread. Capture commands run on a
//!   worker thread, where a direct AppKit call traps with "Must only be used
//!   from the main thread" (SIGTRAP). Every call is therefore marshalled to the
//!   main thread.

#[cfg(windows)]
use crate::app_state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "macos")]
use std::time::Duration;
use tauri::{AppHandle, Manager};

static OVERLAY_VISIBLE: AtomicBool = AtomicBool::new(true);

#[cfg(target_os = "macos")]
fn on_main_thread() -> bool {
    unsafe {
        crate::macos::send(
            crate::macos::class("NSThread"),
            "isMainThread",
            std::ptr::null_mut(),
        ) as isize
            != 0
    }
}

/// Run `action` on the main thread, waiting for it when called from a worker.
#[cfg(target_os = "macos")]
fn on_main(app: &AppHandle, action: impl FnOnce() + Send + 'static) {
    if on_main_thread() {
        action();
        return;
    }

    let (tx, rx) = std::sync::mpsc::channel();
    if app
        .run_on_main_thread(move || {
            action();
            let _ = tx.send(());
        })
        .is_ok()
    {
        let _ = rx.recv_timeout(Duration::from_secs(2));
    }
}

#[cfg(target_os = "macos")]
fn ns_application() -> *mut std::ffi::c_void {
    unsafe {
        crate::macos::send(
            crate::macos::class("NSApplication"),
            "sharedApplication",
            std::ptr::null_mut(),
        )
    }
}

/// Show the overlay without stealing keyboard focus from the app underneath.
///
/// `activateIgnoringOtherApps` + `makeKeyAndOrderFront` activated TrueHalo and
/// pulled focus away from the window the user was working in. The overlay is
/// meant to appear on top while the other app keeps focus; clicking the overlay
/// still focuses it the normal way.
pub fn show(app: &AppHandle) {
    OVERLAY_VISIBLE.store(true, Ordering::Relaxed);

    #[cfg(target_os = "macos")]
    {
        let app_handle = app.clone();
        on_main(app, move || unsafe {
            let ns_app = ns_application();
            if !ns_app.is_null() {
                // A hidden app must be unhidden first, otherwise the window
                // cannot be ordered in at all.
                crate::macos::send(ns_app, "unhide:", std::ptr::null_mut());
            }

            if let Some(window) = app_handle.get_webview_window("main") {
                if let Ok(ns_window) = window.ns_window() {
                    // Order the window to the front without making TrueHalo the
                    // active app: focus stays where the user left it.
                    crate::macos::send(ns_window, "orderFrontRegardless", std::ptr::null_mut());
                }
            }
        });
    }

    #[cfg(not(target_os = "macos"))]
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        // Re-apply native capture protection after showing the window so that
        // tao does not reset DWM transparency / shadow / acrylic back to defaults.
        #[cfg(windows)]
        {
            if let Some(state) = app.try_state::<AppState>() {
                let state_guard = state.db.get_preference("exclude_screen_recording");
                let exclude = state_guard.ok().flatten().map(|v| v == "true").unwrap_or(true);
                if exclude {
                    let _ = crate::commands::window_commands::apply_capture_exclusion(app, true);
                }
            }
        }
    }
}

/// Hide the overlay (including the menu bar/dock activation) without closing it.
pub fn hide(app: &AppHandle) {
    OVERLAY_VISIBLE.store(false, Ordering::Relaxed);

    #[cfg(target_os = "macos")]
    on_main(app, || unsafe {
        let ns_app = ns_application();
        if !ns_app.is_null() {
            crate::macos::send(ns_app, "hide:", std::ptr::null_mut());
        }
    });

    #[cfg(not(target_os = "macos"))]
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

/// Whether the overlay is currently on screen.
pub fn is_visible(app: &AppHandle) -> bool {
    #[cfg(target_os = "macos")]
    {
        let _ = app;

        if !on_main_thread() {
            return OVERLAY_VISIBLE.load(Ordering::Relaxed);
        }

        let visible = unsafe {
            let ns_app = ns_application();
            if ns_app.is_null() {
                return OVERLAY_VISIBLE.load(Ordering::Relaxed);
            }
            (crate::macos::send(ns_app, "isHidden", std::ptr::null_mut()) as isize) == 0
        };
        tracing::debug!(
            "overlay state: ns_hidden={} cached_visible={}",
            !visible,
            OVERLAY_VISIBLE.load(Ordering::Relaxed)
        );
        OVERLAY_VISIBLE.store(visible, Ordering::Relaxed);
        visible
    }

    #[cfg(not(target_os = "macos"))]
    {
        app.get_webview_window("main")
            .and_then(|w| w.is_visible().ok())
            .unwrap_or(false)
    }
}
