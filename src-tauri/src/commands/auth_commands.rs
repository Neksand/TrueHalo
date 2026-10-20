use crate::auth::keychain;
use serde::Serialize;
use tauri::command;

#[command]
pub fn save_api_key(provider: String, key: String) -> Result<(), String> {
    if key.trim().is_empty() {
        return Err("API key cannot be empty".to_string());
    }
    keychain::save_credential(&provider, key.trim()).map_err(|e| e.to_string())
}

#[command]
pub fn delete_api_key(provider: String) -> Result<(), String> {
    keychain::delete_credential(&provider).map_err(|e| e.to_string())
}

#[command]
pub fn has_api_key(provider: String) -> bool {
    keychain::has_credential(&provider)
}

#[derive(Debug, Clone, Serialize)]
pub struct KeychainRepairResult {
    /// Providers were inspected.
    pub checked: usize,
    /// Providers whose credential is now owned by the current signature.
    pub repaired: usize,
    /// Providers that could not be rewritten (the user cancelled the dialog).
    pub failed: Vec<String>,
    /// True when at least one key still exists.
    pub has_any_key: bool,
}

/// Rewrite existing credentials so macOS stops asking for the login password.
///
/// Keychain items created by an older (unsigned / ad-hoc) build have an ACL that
/// points at that vanished signature. macOS then asks for the login password on
/// every read. Re-creating the item binds it to the current, stable certificate.
#[command]
pub fn repair_keychain_access() -> KeychainRepairResult {
    const PROVIDERS: [&str; 3] = ["openrouter", "gemini", "openai"];

    let results = keychain::repair_credentials(&PROVIDERS);
    let failed: Vec<String> = results
        .iter()
        .filter(|(_, ok)| !ok)
        .map(|(provider, _)| provider.clone())
        .collect();

    KeychainRepairResult {
        checked: results.len(),
        repaired: results.iter().filter(|(_, ok)| *ok).count(),
        has_any_key: PROVIDERS
            .iter()
            .any(|provider| keychain::has_credential(provider)),
        failed,
    }
}
