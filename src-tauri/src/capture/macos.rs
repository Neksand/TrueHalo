//! macOS screen capture through the system `screencapture` tool.

use super::CaptureOptions;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Capture the screen (or an interactive region) into `output`.
///
/// Returns `false` when the user cancelled the interactive selection, in which
/// case no file is produced.
pub fn grab(options: &CaptureOptions, output: &Path) -> Result<bool> {
    let mut cmd = Command::new("/usr/sbin/screencapture");
    // -x: do not play the shutter sound
    cmd.arg("-x");
    cmd.arg("-t").arg("png");

    if options.interactive {
        // -i: interactive marquee region or window selection
        cmd.arg("-i");
    } else {
        // -m: main monitor, -C: include the cursor
        cmd.arg("-m");
        if options.show_cursor {
            cmd.arg("-C");
        }
    }

    cmd.arg(output);

    tracing::info!(
        "screencapture: starting (interactive={})",
        options.interactive
    );
    let status = cmd
        .status()
        .context("Failed to execute /usr/sbin/screencapture")?;

    let captured = std::fs::metadata(output)
        .map(|metadata| metadata.len() > 0)
        .unwrap_or(false);
    tracing::info!(
        "screencapture: exited with {:?}, file exists={}",
        status.code(),
        captured
    );

    Ok(captured)
}
