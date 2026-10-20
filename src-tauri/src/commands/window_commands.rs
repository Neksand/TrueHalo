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

/// Toggle the native macOS backdrop blur (vibrancy) behind the translucent UI.
#[command]
pub fn set_vibrancy(app: AppHandle, enabled: bool) -> Result<bool, String> {
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
