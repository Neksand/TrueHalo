use crate::auth::keychain;
use serde::Serialize;
use tauri::command;

// Keychain reads can raise the macOS password dialog and block until the user
// answers. These commands are therefore async and run on the blocking pool:
// a synchronous command would block the main thread and leave the transparent
// window blank while the dialog is up.

#[command]
pub async fn save_api_key(provider: String, key: String) -> Result<(), String> {
    if key.trim().is_empty() {
        return Err("API key cannot be empty".to_string());
    }
    let key = key.trim().to_string();

    // Header values cannot contain control characters. Rejecting them here
    // gives a clear message instead of a request sent without Authorization.
    if key.chars().any(|c| c.is_control()) {
        return Err("API key contains control characters; paste it again".to_string());
    }

    let expected = key.clone();
    let read_provider = provider.clone();

    // Write and read back on the same thread: some platforms do not guarantee
    // cross-thread ordering for credential store operations.
    let stored = tokio::task::spawn_blocking(move || {
        keychain::save_credential(&provider, &key)?;
        keychain::get_credential(&read_provider)
    })
    .await
    .map_err(|e| e.to_string())?;

    match stored {
        Ok(actual) if actual == expected => Ok(()),
        Ok(_) => Err(
            "The key was written to the credential store but read back differently. Try saving it again."
                .to_string(),
        ),
        Err(err) => {
            tracing::warn!("Saved API key could not be read back: {}", err);
            Err(format!(
                "The key was written but could not be read back ({err}). Try saving it again."
            ))
        }
    }
}

#[command]
pub async fn delete_api_key(provider: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        keychain::delete_credential(&provider).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[command]
pub async fn has_api_key(provider: String) -> bool {
    // Local servers need no API key.
    if provider.eq_ignore_ascii_case("local") {
        return true;
    }
    tokio::task::spawn_blocking(move || keychain::has_credential(&provider))
        .await
        .unwrap_or(false)
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
pub async fn repair_keychain_access() -> KeychainRepairResult {
    tokio::task::spawn_blocking(|| {
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
    })
    .await
    .unwrap_or_else(|_| KeychainRepairResult {
        checked: 0,
        repaired: 0,
        failed: vec![],
        has_any_key: false,
    })
}
