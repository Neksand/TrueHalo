//! Linux screen capture through the Desktop Portal.
//!
//! The `org.freedesktop.portal.Screenshot` API works on Wayland and X11
//! alike (GNOME, KDE and others) and shows the system's own region picker
//! for interactive captures — no external tools needed. It requires
//! `xdg-desktop-portal` running in the session; without it the capture fails
//! with a message saying so instead of failing silently.

use super::CaptureOptions;
use anyhow::{Context, Result};
use std::path::Path;

pub fn grab(options: &CaptureOptions, output: &Path) -> Result<bool> {
    // `grab` is synchronous (it runs on a blocking thread), while the portal
    // API is async: bridge with a throwaway current-thread runtime.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to start the portal runtime")?;
    rt.block_on(grab_async(options, output))
}

async fn grab_async(options: &CaptureOptions, output: &Path) -> Result<bool> {
    let response = ashpd::desktop::screenshot::Screenshot::request()
        .interactive(options.interactive)
        .modal(true)
        .send()
        .await
        .context("Screen capture needs xdg-desktop-portal running in your session")?
        .response()
        .context("The screenshot request was dismissed")?;

    let uri = response.uri().to_string();
    let path = uri
        .strip_prefix("file://")
        .context("The portal returned an unexpected screenshot location")?;
    let path = url_decode(path);
    std::fs::copy(&path, output).context("Failed to read the portal screenshot")?;
    Ok(true)
}

/// Minimal percent-decoding for `file://` URIs (`%20` etc.).
fn url_decode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut bytes = input.as_bytes().iter();
    while let Some(&b) = bytes.next() {
        if b == b'%' {
            let hi = bytes.next().copied().unwrap_or(b'0');
            let lo = bytes.next().copied().unwrap_or(b'0');
            let hex = |c: u8| (c as char).to_digit(16).unwrap_or(0) as u8;
            out.push((hex(hi) << 4 | hex(lo)) as char);
        } else {
            out.push(b as char);
        }
    }
    out
}
