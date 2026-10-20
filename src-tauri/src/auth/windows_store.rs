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
