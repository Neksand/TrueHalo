use crate::storage::db::DatabaseManager;
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub const PREF_TOGGLE: &str = "hotkey_toggle";
pub const PREF_REGION: &str = "hotkey_region";
pub const PREF_FULLSCREEN: &str = "hotkey_fullscreen";

pub const DEFAULT_TOGGLE: &str = "Alt+Space";
pub const DEFAULT_REGION: &str = "Alt+Shift+Digit2";
pub const DEFAULT_FULLSCREEN: &str = "Alt+Shift+Digit3";

/// The three global shortcuts TrueHalo owns. Stored as Tauri accelerator
/// strings (`Alt+Shift+Digit2`), displayed with platform glyphs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotkeyConfig {
    pub toggle: String,
    pub region: String,
    pub fullscreen: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            toggle: DEFAULT_TOGGLE.to_string(),
            region: DEFAULT_REGION.to_string(),
            fullscreen: DEFAULT_FULLSCREEN.to_string(),
        }
    }
}

impl HotkeyConfig {
    pub fn from_db(db: &DatabaseManager) -> Self {
        let defaults = Self::default();
        Self {
            toggle: read_pref(db, PREF_TOGGLE).unwrap_or(defaults.toggle),
            region: read_pref(db, PREF_REGION).unwrap_or(defaults.region),
            fullscreen: read_pref(db, PREF_FULLSCREEN).unwrap_or(defaults.fullscreen),
        }
    }

    pub fn save(&self, db: &DatabaseManager) -> Result<(), String> {
        db.set_preference(PREF_TOGGLE, &self.toggle)
            .map_err(|e| e.to_string())?;
        db.set_preference(PREF_REGION, &self.region)
            .map_err(|e| e.to_string())?;
        db.set_preference(PREF_FULLSCREEN, &self.fullscreen)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Reject empty or duplicated shortcuts before touching the OS.
    pub fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("Show / hide", &self.toggle),
            ("Region capture", &self.region),
            ("Full screen capture", &self.fullscreen),
        ] {
            if value.trim().is_empty() {
                return Err(format!("{name} needs a shortcut"));
            }
        }

        if self.toggle == self.region || self.toggle == self.fullscreen || self.region == self.fullscreen
        {
            return Err("Each action needs a different shortcut".to_string());
        }

        Ok(())
    }

    pub fn toggle_label(&self) -> String {
        display(&self.toggle)
    }

    pub fn region_label(&self) -> String {
        display(&self.region)
    }

    pub fn fullscreen_label(&self) -> String {
        display(&self.fullscreen)
    }
}

fn read_pref(db: &DatabaseManager, key: &str) -> Option<String> {
    db.get_preference(key)
        .ok()
        .flatten()
        .filter(|value| !value.trim().is_empty())
}

/// Register all three shortcuts, replacing any previously registered ones.
///
/// If a later registration fails the caller is responsible for restoring the
/// previous configuration; `set_hotkeys` does exactly that.
pub fn register(app: &AppHandle, config: &HotkeyConfig) -> Result<(), String> {
    config.validate()?;

    let shortcuts = app.global_shortcut();
    let _ = shortcuts.unregister_all();

    let toggle_handle = app.clone();
    shortcuts
        .on_shortcut(config.toggle.as_str(), move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                crate::toggle_overlay_window(&toggle_handle);
            }
        })
        .map_err(|e| format!("{} could not be registered: {e}", config.toggle_label()))?;

    let region_handle = app.clone();
    shortcuts
        .on_shortcut(config.region.as_str(), move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let _ = region_handle.emit("hotkey:capture-region", ());
            }
        })
        .map_err(|e| format!("{} could not be registered: {e}", config.region_label()))?;

    let fullscreen_handle = app.clone();
    shortcuts
        .on_shortcut(
            config.fullscreen.as_str(),
            move |_app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    let _ = fullscreen_handle.emit("hotkey:capture-fullscreen", ());
                }
            },
        )
        .map_err(|e| format!("{} could not be registered: {e}", config.fullscreen_label()))?;

    Ok(())
}

/// Human label for a Tauri accelerator: `Alt+Shift+Digit2` → `⌥⇧2` on macOS,
/// `Alt+Shift+2` elsewhere.
pub fn display(accelerator: &str) -> String {
    let parts: Vec<String> = accelerator
        .split('+')
        .map(|part| key_label(part.trim()))
        .collect();

    #[cfg(target_os = "macos")]
    {
        parts
            .into_iter()
            .map(|part| match part.as_str() {
                "Alt" => "⌥".to_string(),
                "Shift" => "⇧".to_string(),
                "Control" | "Ctrl" => "⌃".to_string(),
                "Super" | "Command" | "Cmd" => "⌘".to_string(),
                _ => part,
            })
            .collect()
    }

    #[cfg(not(target_os = "macos"))]
    {
        parts.join("+")
    }
}

fn key_label(part: &str) -> String {
    if let Some(rest) = part.strip_prefix("Digit") {
        return rest.to_string();
    }
    if let Some(rest) = part.strip_prefix("Key") {
        return rest.to_string();
    }
    part.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        assert!(HotkeyConfig::default().validate().is_ok());
    }

    #[test]
    fn duplicates_are_rejected() {
        let config = HotkeyConfig {
            toggle: "Alt+Space".into(),
            region: "Alt+Space".into(),
            fullscreen: "Alt+Shift+Digit3".into(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn empty_shortcut_is_rejected() {
        let config = HotkeyConfig {
            toggle: " ".into(),
            region: DEFAULT_REGION.into(),
            fullscreen: DEFAULT_FULLSCREEN.into(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn labels_are_platform_readable() {
        assert!(display(DEFAULT_REGION).contains('2'));
        assert!(!display(DEFAULT_REGION).contains("Digit"));
    }
}
