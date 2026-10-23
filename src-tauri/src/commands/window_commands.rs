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
    apply_capture_exclusion(&app, exclude)
}

/// Windows: hide the window from screen capture APIs.
///
/// `WDA_EXCLUDEFROMCAPTURE` removes the window from recordings entirely, so the
/// desktop behind it shows through. It exists since Windows 10 2004 (build
/// 19041); on older builds the OS silently degrades it to `WDA_MONITOR`, which
/// paints a black rectangle over the content underneath. We therefore check the
/// build first and simply leave the window untouched when exclusion is not
/// available, instead of producing that black box.
#[cfg(windows)]
fn apply_capture_exclusion(app: &AppHandle, exclude: bool) -> Result<bool, String> {
    const MIN_BUILD_FOR_EXCLUSION: u32 = 19_041;

    let build = windows_build_number();
    let Some(window) = app.get_webview_window("main") else {
        return Ok(false);
    };

    if exclude && build < MIN_BUILD_FOR_EXCLUSION {
        // Make sure no stale affinity from an earlier session survives.
        let _ = window.set_content_protected(false);
        tracing::warn!(
            "Recording protection requires Windows 10 2004+ (build {} detected); leaving the window visible in captures",
            build
        );
        return Ok(false);
    }

    match window.set_content_protected(exclude) {
        Ok(()) => {
            tracing::info!(
                "Windows display affinity: exclude={} (build {})",
                exclude,
                build
            );
            Ok(true)
        }
        Err(err) => {
            tracing::warn!("Could not set the Windows display affinity: {}", err);
            Ok(false)
        }
    }
}

/// macOS: `NSWindowSharingType`.
#[cfg(target_os = "macos")]
fn apply_capture_exclusion(app: &AppHandle, exclude: bool) -> Result<bool, String> {
    if let Some(window) = app.get_webview_window("main") {
        // NSWindowSharingType:
        // 0 = NSWindowSharingNone (invisible to other apps' recordings & screenshots)
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
    Ok(false)
}

/// Real Windows build number, unaffected by the app compatibility manifest.
#[cfg(windows)]
fn windows_build_number() -> u32 {
    #[repr(C)]
    struct OsVersionInfoW {
        size: u32,
        major: u32,
        minor: u32,
        build: u32,
        platform: u32,
        csd_version: [u16; 128],
    }

    #[link(name = "ntdll")]
    extern "system" {
        fn RtlGetVersion(info: *mut OsVersionInfoW) -> i32;
    }

    let mut info = OsVersionInfoW {
        size: std::mem::size_of::<OsVersionInfoW>() as u32,
        major: 0,
        minor: 0,
        build: 0,
        platform: 0,
        csd_version: [0; 128],
    };

    // RtlGetVersion returns STATUS_SUCCESS (0).
    let status = unsafe { RtlGetVersion(&mut info) };
    if status == 0 {
        info.build
    } else {
        0
    }
}
