use security_framework::item::{ItemClass, ItemSearchOptions};
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};
use std::fmt;

const SERVICE_NAME: &str = "com.truehalo.desktop";

#[derive(Debug)]
pub enum KeychainError {
    SecError(security_framework::base::Error),
    Utf8Error(std::string::FromUtf8Error),
    NotFound,
}

impl fmt::Display for KeychainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeychainError::SecError(e) => write!(f, "macOS Keychain error: {}", e),
            KeychainError::Utf8Error(e) => write!(f, "Invalid UTF-8 in keychain payload: {}", e),
            KeychainError::NotFound => write!(f, "Credential not found in macOS Keychain"),
        }
    }
}

impl std::error::Error for KeychainError {}

impl From<security_framework::base::Error> for KeychainError {
    fn from(err: security_framework::base::Error) -> Self {
        // errSecItemNotFound = -25300
        if err.code() == -25300 {
            KeychainError::NotFound
        } else {
            KeychainError::SecError(err)
        }
    }
}

fn account_for(provider: &str) -> String {
    format!("provider:{}", provider.to_lowercase())
}

/// Store a credential.
///
/// The item is always recreated (delete + add) instead of updated: macOS binds
/// each keychain item's ACL to the exact code signature that created it, and an
/// update keeps the old ACL. Recreating makes the item belong to the *current*
/// (stable, certificate-based) signature, so macOS stops asking for the login
/// password on every access — including after future rebuilds.
pub fn save_credential(provider: &str, secret: &str) -> Result<(), KeychainError> {
    let account = account_for(provider);

    // Ignore "not found" — we only care that no stale item with an old ACL is left.
    let _ = delete_generic_password(SERVICE_NAME, &account);

    set_generic_password(SERVICE_NAME, &account, secret.as_bytes())?;
    Ok(())
}

pub fn get_credential(provider: &str) -> Result<String, KeychainError> {
    let account = account_for(provider);
    let bytes = get_generic_password(SERVICE_NAME, &account)?;
    String::from_utf8(bytes).map_err(KeychainError::Utf8Error)
}

pub fn delete_credential(provider: &str) -> Result<(), KeychainError> {
    let account = account_for(provider);
    match delete_generic_password(SERVICE_NAME, &account) {
        Ok(()) => Ok(()),
        Err(e) if e.code() == -25300 => Ok(()), // Already not found, treat as ok
        Err(e) => Err(KeychainError::from(e)),
    }
}

/// Whether a credential exists — **without** asking the user for a password.
///
/// `get_generic_password` requests the secret data, which triggers the keychain
/// ACL check (the "TrueHalo wants to use your confidential information" /
/// password dialog). Querying only the item *attributes* never touches the
/// protected data, so existence checks stay silent.
pub fn has_credential(provider: &str) -> bool {
    let account = account_for(provider);

    let result = ItemSearchOptions::new()
        .class(ItemClass::generic_password())
        .service(SERVICE_NAME)
        .account(&account)
        .load_attributes(true)
        .limit(1)
        .search();

    match result {
        Ok(items) => !items.is_empty(),
        Err(_) => false,
    }
}

/// Re-create every stored credential so its ACL points at the current signature.
///
/// Reading the old value may still raise one system dialog (macOS protects data
/// created by a previous signature). After this runs, the item is owned by the
/// current signature and no further prompts appear.
pub fn repair_credentials(providers: &[&str]) -> Vec<(String, bool)> {
    providers
        .iter()
        .map(|provider| {
            if !has_credential(provider) {
                return (provider.to_string(), true);
            }

            match get_credential(provider) {
                Ok(secret) => {
                    // Recreate with a fresh ACL bound to this build's signature.
                    if save_credential(provider, &secret).is_ok() {
                        (provider.to_string(), true)
                    } else {
                        (provider.to_string(), false)
                    }
                }
                Err(_) => (provider.to_string(), false),
            }
        })
        .collect()
}
