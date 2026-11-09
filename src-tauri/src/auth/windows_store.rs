//! Windows Credential Manager storage.
//!
//! Mirrors the macOS keychain API so the command layer stays platform-neutral.
//! Credentials are stored as generic credentials under the application service
//! name, one entry per provider.

use keyring::Entry;
use std::fmt;

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

fn map_error(err: keyring::Error) -> KeychainError {
    match err {
        keyring::Error::NoEntry => KeychainError::NotFound,
        keyring::Error::NoStorageAccess(inner) => KeychainError::Store(format!(
            "Windows Credential Manager is unavailable in this logon session ({inner}). \
             Launch TrueHalo as your normal desktop user (not elevated, and not from a \
             service or SSH session), then try again."
        )),
        other => KeychainError::Store(other.to_string()),
    }
}

fn account_for(provider: &str) -> String {
    format!("provider:{}", provider.to_lowercase())
}

fn entry_for(provider: &str) -> Result<Entry, KeychainError> {
    Entry::new(SERVICE_NAME, &account_for(provider)).map_err(map_error)
}

/// Store (or replace) the API key for `provider`.
pub fn save_credential(provider: &str, secret: &str) -> Result<(), KeychainError> {
    entry_for(provider)?.set_password(secret).map_err(map_error)
}

/// Read the API key for `provider`.
pub fn get_credential(provider: &str) -> Result<String, KeychainError> {
    entry_for(provider)?.get_password().map_err(map_error)
}

/// Remove the stored key. Missing entries are treated as already deleted.
pub fn delete_credential(provider: &str) -> Result<(), KeychainError> {
    match entry_for(provider)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(map_error(err)),
    }
}

/// Whether a credential exists. Reads the secret once; Windows does not prompt.
pub fn has_credential(provider: &str) -> bool {
    matches!(entry_for(provider), Ok(entry) if entry.get_password().is_ok())
}

/// Rewrite every stored credential so it belongs to the current executable.
///
/// Windows does not bind credentials to a code signature, so this is mostly a
/// no-op that keeps the shared "Fix" action meaningful on both platforms.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trips a credential through the Windows store. Skips when the
    /// session has no credential set (CI service sessions), which is exactly
    /// the case the UI cannot currently explain.
    #[test]
    fn credential_roundtrip() {
        let provider = format!("test-{}", uuid::Uuid::new_v4());

        match save_credential(&provider, "test-value") {
            Ok(()) => {}
            // GitHub runners can execute in a session without a credential
            // set; skip there instead of blocking every build.
            Err(KeychainError::Store(message)) if message.contains("unavailable") => {
                eprintln!("Credential Manager unavailable, skipping: {message}");
                return;
            }
            Err(err) => panic!("save_credential failed: {err}"),
        }

        assert_eq!(get_credential(&provider).unwrap(), "test-value");
        assert!(has_credential(&provider));

        delete_credential(&provider).unwrap();
        assert!(matches!(
            get_credential(&provider),
            Err(KeychainError::NotFound)
        ));
    }
}
