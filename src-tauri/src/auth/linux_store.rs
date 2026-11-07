//! Linux credential storage: Secret Service with a file fallback.
//!
//! Mirrors the other backends' API so the command layer stays platform-neutral.
//! The primary store is the freedesktop Secret Service (GNOME Keyring,
//! KWallet) through the `keyring` crate. When no D-Bus session is available
//! (headless servers, minimal window managers without a keyring daemon),
//! credentials fall back to files with `0600` permissions under
//! `~/.local/share/truehalo/credentials/`. The file store is only consulted
//! when the Secret Service itself is unusable — never as a second opinion.

use keyring::Entry;
use std::fmt;
use std::path::PathBuf;

const SERVICE_NAME: &str = "com.truehalo.desktop";

#[derive(Debug)]
pub enum KeychainError {
    Store(String),
    NotFound,
}

impl fmt::Display for KeychainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeychainError::Store(e) => write!(f, "Credential store error: {}", e),
            KeychainError::NotFound => write!(f, "Credential not found in the credential store"),
        }
    }
}

impl std::error::Error for KeychainError {}

fn account_for(provider: &str) -> String {
    format!("provider:{}", provider.to_lowercase())
}

fn fallback_path(provider: &str) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(
        PathBuf::from(home)
            .join(".local/share/truehalo/credentials")
            .join(account_for(provider)),
    )
}

fn read_fallback(provider: &str) -> Result<String, KeychainError> {
    let path = fallback_path(provider).ok_or(KeychainError::NotFound)?;
    std::fs::read_to_string(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            KeychainError::NotFound
        } else {
            KeychainError::Store(e.to_string())
        }
    })
}

fn write_fallback(provider: &str, secret: &str) -> Result<(), KeychainError> {
    use std::os::unix::fs::PermissionsExt;

    let path = fallback_path(provider).ok_or(KeychainError::Store(
        "HOME is not set; cannot store the credential".to_string(),
    ))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| KeychainError::Store(e.to_string()))?;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| KeychainError::Store(e.to_string()))?;
    }
    std::fs::write(&path, secret).map_err(|e| KeychainError::Store(e.to_string()))?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| KeychainError::Store(e.to_string()))?;
    Ok(())
}

fn delete_fallback(provider: &str) {
    if let Some(path) = fallback_path(provider) {
        let _ = std::fs::remove_file(path);
    }
}

/// True when the Secret Service itself is unusable (as opposed to the entry
/// simply being absent). Only then does the file fallback kick in.
fn store_unusable(err: &keyring::Error) -> bool {
    !matches!(err, keyring::Error::NoEntry | keyring::Error::Ambiguous(_))
}

/// Store (or replace) the API key for `provider`.
pub fn save_credential(provider: &str, secret: &str) -> Result<(), KeychainError> {
    let entry = Entry::new(SERVICE_NAME, &account_for(provider))
        .map_err(|e| KeychainError::Store(e.to_string()))?;
    match entry.set_password(secret) {
        Ok(()) => {
            delete_fallback(provider);
            Ok(())
        }
        Err(e) if store_unusable(&e) => write_fallback(provider, secret),
        Err(e) => Err(KeychainError::Store(e.to_string())),
    }
}

/// Read the API key for `provider`.
pub fn get_credential(provider: &str) -> Result<String, KeychainError> {
    let entry = Entry::new(SERVICE_NAME, &account_for(provider))
        .map_err(|e| KeychainError::Store(e.to_string()))?;
    match entry.get_password() {
        Ok(secret) => Ok(secret),
        Err(keyring::Error::NoEntry) => Err(KeychainError::NotFound),
        Err(e) if store_unusable(&e) => read_fallback(provider),
        Err(e) => Err(KeychainError::Store(e.to_string())),
    }
}

/// Remove the stored key. Missing entries are treated as already deleted.
pub fn delete_credential(provider: &str) -> Result<(), KeychainError> {
    delete_fallback(provider);
    let entry = Entry::new(SERVICE_NAME, &account_for(provider))
        .map_err(|e| KeychainError::Store(e.to_string()))?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) if store_unusable(&e) => Ok(()),
        Err(e) => Err(KeychainError::Store(e.to_string())),
    }
}

/// Whether a credential exists. Reads the secret once; the Secret Service
/// does not prompt for stored items, and the file fallback never prompts.
pub fn has_credential(provider: &str) -> bool {
    match Entry::new(SERVICE_NAME, &account_for(provider)) {
        Ok(entry) => match entry.get_password() {
            Ok(_) => true,
            Err(keyring::Error::NoEntry) => false,
            Err(e) if store_unusable(&e) => read_fallback(provider).is_ok(),
            Err(_) => false,
        },
        Err(_) => read_fallback(provider).is_ok(),
    }
}

/// Rewrite every stored credential so it lives in the working store.
pub fn repair_credentials(providers: &[&str]) -> Vec<(String, bool)> {
    providers
        .iter()
        .map(|provider| {
            let ok = match get_credential(provider) {
                Ok(secret) => save_credential(provider, &secret).is_ok(),
                Err(KeychainError::NotFound) => true,
                Err(_) => false,
            };
            ((*provider).to_string(), ok)
        })
        .collect()
}
