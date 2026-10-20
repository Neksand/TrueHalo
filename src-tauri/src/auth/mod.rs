//! Credential storage.
//!
//! Both backends expose the same small API, so the command layer never needs to
//! know which operating system it is running on.

#[cfg(target_os = "macos")]
pub mod keychain;

#[cfg(windows)]
#[path = "windows_store.rs"]
pub mod keychain;
