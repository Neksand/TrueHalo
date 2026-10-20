use tauri::{command, AppHandle, Manager};

#[command]
pub fn toggle_overlay(app: AppHandle) -> Result<(), String> {
    if app.get_webview_window("main").is_none() {
        return Err("Main window not found".to_string());
    }

    if crate::overlay::is_visible(&app) {
        crate::overlay::hide(&app);
    } else {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.unminimize();
        }
        crate::overlay::show(&app);
    }
    Ok(())
}

#[command]
pub fn quick_hide(app: AppHandle) -> Result<(), String> {
    if app.get_webview_window("main").is_none() {
        return Err("Main window not found".to_string());
    }
    crate::overlay::hide(&app);
    Ok(())
}

#[command]
pub fn show_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        crate::overlay::show(&app);
        Ok(())
    } else {
        Err("Main window not found".to_string())
    }
}

#[command]
pub fn set_always_on_top(app: AppHandle, always_on_top: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window
            .set_always_on_top(always_on_top)
            .map_err(|e| e.to_string())
    } else {
        Err("Main window not found".to_string())
    }
}

/// Toggle the native backdrop blur (vibrancy / acrylic) behind the translucent UI.
#[command]
pub fn set_vibrancy(app: AppHandle, enabled: bool) -> Result<bool, String> {
    #[cfg(windows)]
    {
        use window_vibrancy::{apply_acrylic, clear_acrylic};

        if let Some(window) = app.get_webview_window("main") {
            let result: Result<(), window_vibrancy::Error> = if enabled {
                // Dark acrylic tint that matches the overlay theme.
                apply_acrylic(&window, Some((18, 18, 18, 160)))
            } else {
                clear_acrylic(&window)
            };

            match result {
                Ok(()) => {
                    tracing::info!("Windows window acrylic set to: {}", enabled);
                    return Ok(true);
                }
                Err(err) => {
                    tracing::warn!("Could not update Windows acrylic: {}", err);
                    return Ok(false);
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{
            apply_vibrancy, clear_vibrancy, NSVisualEffectMaterial, NSVisualEffectState,
        };

        if let Some(window) = app.get_webview_window("main") {
            let result: Result<(), window_vibrancy::Error> = if enabled {
                apply_vibrancy(
                    &window,
                    // `hudWindow` keeps text readable on dark surfaces.
                    NSVisualEffectMaterial::HudWindow,
                    Some(NSVisualEffectState::Active),
                    Some(16.0),
                )
            } else {
                clear_vibrancy(&window).map(|_| ())
            };

            match result {
                Ok(()) => {
                    tracing::info!("macOS window vibrancy set to: {}", enabled);
                    return Ok(true);
                }
                Err(err) => {
                    tracing::warn!("Could not update macOS vibrancy: {}", err);
                    return Ok(false);
                }
            }
        }
    }

    let _ = (app, enabled);
    Ok(false)
}

/// Hide the app from the Dock / ⌘Tab switcher (macOS accessory activation policy).
#[command]
pub fn set_hide_from_dock(app: AppHandle, hide: bool) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        use tauri::ActivationPolicy;

        let policy = if hide {
            ActivationPolicy::Accessory
        } else {
            ActivationPolicy::Regular
        };

        app.set_activation_policy(policy)
            .map_err(|e| e.to_string())?;
        tracing::info!("macOS activation policy set, hide_from_dock={}", hide);
        Ok(true)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, hide);
        Ok(false)
    }
}

#[command]
pub fn set_screen_recording_protection(app: AppHandle, exclude: bool) -> Result<bool, String> {
    #[cfg(windows)]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use std::ffi::c_void;

        // WDA_NONE = 0x00000000, WDA_EXCLUDEFROMCAPTURE = 0x00000011 (Windows 10 2004+).
        const WDA_NONE: u32 = 0x0000_0000;
        const WDA_EXCLUDEFROMCAPTURE: u32 = 0x0000_0011;

        #[link(name = "user32")]
        extern "system" {
            fn SetWindowDisplayAffinity(hwnd: *mut c_void, affinity: u32) -> i32;
        }

        if let Some(window) = app.get_webview_window("main") {
            if let Ok(handle) = window.window_handle() {
                if let RawWindowHandle::Win32(win32) = handle.as_raw() {
                    let affinity = if exclude {
                        WDA_EXCLUDEFROMCAPTURE
                    } else {
                        WDA_NONE
                    };
                    let ok = unsafe {
                        SetWindowDisplayAffinity(win32.hwnd.get() as *mut c_void, affinity) != 0
                    };
                    tracing::info!("Windows SetWindowDisplayAffinity({}) -> {}", affinity, ok);
                    return Ok(ok);
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(window) = app.get_webview_window("main") {
            // NSWindowSharingType:
            // 0 = NSWindowSharingNone (invisible to other apps' screen recordings & screenshots)
            // 1 = NSWindowSharingReadOnly (normal recording)
            let sharing_type: isize = if exclude { 0 } else { 1 };

            if let Ok(ns_win_ptr) = window.ns_window() {
                unsafe {
                    crate::macos::send(
                        ns_win_ptr,
                        "setSharingType:",
                        sharing_type as *mut std::ffi::c_void,
                    );
                }
                tracing::info!("macOS NSWindow.sharingType set to: {}", sharing_type);
                return Ok(true);
            }
        }
    }
    Ok(false)
}
