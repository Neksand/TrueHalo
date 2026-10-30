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

/// Let mouse events pass through the overlay (click-through mode).
///
/// The window stays visible but no longer takes clicks; the global hotkeys
/// still work, so the user can toggle it back without a mouse.
#[command]
pub fn set_click_through(app: AppHandle, ignore: bool) -> Result<bool, String> {
    if let Some(window) = app.get_webview_window("main") {
        window
            .set_ignore_cursor_events(ignore)
            .map_err(|e| e.to_string())?;
        tracing::info!("click-through set to: {}", ignore);
        Ok(true)
    } else {
        Err("Main window not found".to_string())
    }
}

/// Hide the window from the taskbar, Alt+Tab and Task View (Windows).
///
/// macOS covers this with the accessory activation policy; on Windows the
/// window is marked as a tool window instead.
#[command]
pub fn set_hide_from_taskbar(app: AppHandle, hide: bool) -> Result<bool, String> {
    if let Some(window) = app.get_webview_window("main") {
        window
            .set_skip_taskbar(hide)
            .map_err(|e| e.to_string())?;
        tracing::info!("skip taskbar set to: {}", hide);
        Ok(true)
    } else {
        Err("Main window not found".to_string())
    }
}

/// Keep the overlay out of Mission Control and the window cycle (macOS).
///
/// `Transient` windows are hidden while Mission Control is active,
/// `Stationary` keeps them out of the window list, and `IgnoresCycle`
/// drops them from Cmd+Tab. Applied on top of the accessory policy.
#[cfg(target_os = "macos")]
pub fn apply_macos_stealth(app: &AppHandle) {
    use std::ffi::c_void;

    const CAN_JOIN_ALL_SPACES: isize = 1;
    const TRANSIENT: isize = 1 << 3;
    const STATIONARY: isize = 1 << 4;
    const IGNORES_CYCLE: isize = 1 << 6;

    if let Some(window) = app.get_webview_window("main") {
        if let Ok(ns_win) = window.ns_window() {
            unsafe {
                crate::macos::send(
                    ns_win,
                    "setCollectionBehavior:",
                    (CAN_JOIN_ALL_SPACES | TRANSIENT | STATIONARY | IGNORES_CYCLE)
                        as *mut c_void,
                );
            }
            tracing::info!("macOS stealth collection behavior applied");
        }
    }
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
        if hide {
            // Switching to Accessory orders the window out on macOS; bring it
            // back so the overlay does not disappear from the screen.
            crate::overlay::show(&app);
        }
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
///
/// Acrylic blur and the DWM shadow are composited outside the window surface,
/// so they are not covered by the exclusion and can leave a dark rectangle in
/// the recording ("a piece of the screen is missing"). Both are dropped while
/// protection is on and restored by the frontend when it is turned off.
#[cfg(windows)]
pub fn apply_capture_exclusion(app: &AppHandle, exclude: bool) -> Result<bool, String> {
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
            // tao creates transparent windows with DWM blur-behind over an empty
            // region. That composition is not part of the window surface, so the
            // capture still receives the window's alpha and paints a black hole
            // where it was. Turn the OS-level transparency off while protection
            // is on (the UI is forced opaque anyway) and restore it afterwards.
            if let Ok(hwnd) = window.hwnd() {
                set_dwm_transparency(hwnd.0 as isize, !exclude);
            }
            if exclude {
                // Blur-behind samples the desktop and the shadow is a separate
                // DWM visual: neither is excluded from the capture, so they
                // would show up as a dark hole over the desktop.
                let _ = window_vibrancy::clear_acrylic(&window);
                let _ = window.set_shadow(false);
            } else {
                let _ = window.set_shadow(true);
            }
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

/// Toggle the DWM blur-behind composition that makes a tao window transparent.
///
/// With `transparent: false` the window stops contributing per-pixel alpha to
/// the desktop composition, which is what keeps `WDA_EXCLUDEFROMCAPTURE` from
/// leaving a black rectangle in recordings. Re-enabling it restores exactly the
/// state tao sets up at creation (an empty blur region).
#[cfg(windows)]
fn set_dwm_transparency(hwnd: isize, transparent: bool) {
    #[repr(C)]
    struct BlurBehind {
        flags: u32,
        enable: i32,
        blur_region: *mut std::ffi::c_void,
        transition_on_maximized: i32,
    }

    const DWM_BB_ENABLE: u32 = 0x1;
    const DWM_BB_BLURREGION: u32 = 0x2;

    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmEnableBlurBehindWindow(
            hwnd: *mut std::ffi::c_void,
            blur_behind: *const BlurBehind,
        ) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateRectRgn(left: i32, top: i32, right: i32, bottom: i32) -> *mut std::ffi::c_void;
        fn DeleteObject(object: *mut std::ffi::c_void) -> i32;
    }

    let hwnd = hwnd as *mut std::ffi::c_void;

    let result = unsafe {
        if transparent {
            let region = CreateRectRgn(0, 0, -1, -1);
            let blur = BlurBehind {
                flags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
                enable: 1,
                blur_region: region,
                transition_on_maximized: 0,
            };
            let result = DwmEnableBlurBehindWindow(hwnd, &blur);
            if !region.is_null() {
                let _ = DeleteObject(region);
            }
            result
        } else {
            let blur = BlurBehind {
                flags: DWM_BB_ENABLE,
                enable: 0,
                blur_region: std::ptr::null_mut(),
                transition_on_maximized: 0,
            };
            DwmEnableBlurBehindWindow(hwnd, &blur)
        }
    };

    if result < 0 {
        tracing::warn!(
            "DwmEnableBlurBehindWindow failed (transparent={}): 0x{:08x}",
            transparent,
            result
        );
    }
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
