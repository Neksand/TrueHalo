//! Windows screen capture through a small PowerShell + WinForms helper.
//!
//! This keeps the port dependency-free: no extra toolchain, no native DLLs.
//! Full-screen capture grabs the whole virtual desktop; the interactive mode
//! shows a dimmed selection overlay and crops the chosen rectangle.

use super::CaptureOptions;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const SCRIPT: &str = r#"
param([string]$Out, [int]$Interactive)
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

function Save-Region([int]$x, [int]$y, [int]$w, [int]$h) {
    if ($w -le 0 -or $h -le 0) { return }
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($x, $y, 0, 0, $bmp.Size)
    $bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose()
    $bmp.Dispose()
}

if ($Interactive -eq 0) {
    $vs = [System.Windows.Forms.SystemInformation]::VirtualScreen
    Save-Region $vs.Left $vs.Top $vs.Width $vs.Height
    exit 0
}

$form = New-Object System.Windows.Forms.Form
$form.FormBorderStyle = 'None'
$form.StartPosition = 'Manual'
$form.Bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
$form.TopMost = $true
$form.BackColor = 'Black'
$form.Opacity = 0.25
$form.Cursor = 'Cross'
$form.ShowInTaskbar = $false
$form.KeyPreview = $true

$script:start = $null
$script:end = $null

$form.Add_MouseDown({ param($s, $e) $script:start = $e.Location })
$form.Add_MouseMove({
    param($s, $e)
    if ($script:start) { $script:end = $e.Location; $form.Invalidate() }
})
$form.Add_Paint({
    if ($script:start -and $script:end) {
        $x = [Math]::Min($script:start.X, $script:end.X)
        $y = [Math]::Min($script:start.Y, $script:end.Y)
        $w = [Math]::Abs($script:start.X - $script:end.X)
        $h = [Math]::Abs($script:start.Y - $script:end.Y)
        $pen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 255, 90, 60)), 2
        $form.CreateGraphics().DrawRectangle($pen, $x, $y, $w, $h)
        $pen.Dispose()
    }
})
$form.Add_MouseUp({
    param($s, $e)
    $sx = [Math]::Min($script:start.X, $e.Location.X)
    $sy = [Math]::Min($script:start.Y, $e.Location.Y)
    $sw = [Math]::Abs($script:start.X - $e.Location.X)
    $sh = [Math]::Abs($script:start.Y - $e.Location.Y)
    $form.Hide()
    Start-Sleep -Milliseconds 150
    $origin = [System.Windows.Forms.SystemInformation]::VirtualScreen
    Save-Region ($origin.Left + $sx) ($origin.Top + $sy) $sw $sh
    $form.Close()
})
$form.Add_KeyDown({ if ($_.KeyCode -eq 'Escape') { $form.Close() } })

[void]$form.ShowDialog()
"#;

/// Capture the screen (or an interactive region) into `output`.
///
/// Returns `false` when the user cancelled the interactive selection.
pub fn grab(options: &CaptureOptions, output: &Path) -> Result<bool> {
    let script_path =
        std::env::temp_dir().join(format!("truehalo_capture_{}.ps1", uuid::Uuid::new_v4()));
    std::fs::write(&script_path, SCRIPT)
        .context("Failed to write the screen capture helper script")?;

    let mut cmd = Command::new("powershell");
    cmd.arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-WindowStyle")
        .arg("Hidden")
        .arg("-File")
        .arg(&script_path)
        .arg(output)
        .arg(if options.interactive { "1" } else { "0" });

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    tracing::info!(
        "powershell capture: starting (interactive={})",
        options.interactive
    );
    let status = cmd.status().context("Failed to run PowerShell capture")?;
    let _ = std::fs::remove_file(&script_path);

    let captured = std::fs::metadata(output)
        .map(|metadata| metadata.len() > 0)
        .unwrap_or(false);
    tracing::info!(
        "powershell capture: exited with {:?}, file exists={}",
        status.code(),
        captured
    );

    Ok(captured)
}
