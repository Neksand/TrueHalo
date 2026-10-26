use base64::{engine::general_purpose::STANDARD, Engine};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;

/// Recognize text in a captured screenshot with the OS OCR engine.
///
/// macOS uses Vision (`VNRecognizeTextRequest`) through the built-in JXA
/// Objective-C bridge; Windows uses `Windows.Media.Ocr` through PowerShell.
/// Both run entirely on the device — the screenshot never leaves the machine
/// for OCR.
pub async fn recognize(base64_data: &str, media_type: &str) -> Result<String, String> {
    let decoded = STANDARD
        .decode(base64_data)
        .map_err(|e| format!("Could not decode the screenshot: {e}"))?;

    // Windows OCR rejects very large bitmaps and macOS OCR gets slower without
    // gaining accuracy. Downscale only when the capture is oversized.
    let (bytes, extension) = match image::load_from_memory(&decoded) {
        Ok(image) if image.width().max(image.height()) > 2200 => {
            let scale = 2200.0 / image.width().max(image.height()) as f64;
            let width = (image.width() as f64 * scale).round() as u32;
            let height = (image.height() as f64 * scale).round() as u32;
            let resized = image::imageops::resize(
                &image,
                width.max(1),
                height.max(1),
                image::imageops::FilterType::Lanczos3,
            );
            let mut png = Vec::new();
            resized
                .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
                .map_err(|e| format!("Could not prepare the screenshot for OCR: {e}"))?;
            (png, "png")
        }
        _ => (
            decoded,
            if media_type.contains("jpeg") || media_type.contains("jpg") {
                "jpg"
            } else {
                "png"
            },
        ),
    };
    let path = temp_path(extension);

    tokio::fs::write(&path, &bytes)
        .await
        .map_err(|e| format!("Could not stage the screenshot for OCR: {e}"))?;

    let result = recognize_file(&path).await;
    let _ = tokio::fs::remove_file(&path).await;
    result
}

#[cfg(target_os = "macos")]
async fn recognize_file(path: &Path) -> Result<String, String> {
    const SCRIPT: &str = r#"
ObjC.import('Foundation');
ObjC.import('Vision');
function run(argv) {
  const path = argv[0];
  const url = $.NSURL.fileURLWithPath(path);
  const handler = $.VNImageRequestHandler.alloc.initWithURLOptions(url, $());
  if (handler.isNil()) return '';
  const request = $.VNRecognizeTextRequest.alloc.init;
  request.recognitionLevel = 0;
  request.usesLanguageCorrection = true;
  const ok = handler.performRequestsError($.NSArray.arrayWithObject(request), $());
  if (!ok) return '';
  const results = request.results;
  const lines = [];
  for (let i = 0; i < results.count; i++) {
    const observation = results.objectAtIndex(i);
    const candidates = observation.topCandidates(1);
    if (candidates.count > 0) {
      lines.push(candidates.objectAtIndex(0).string.js);
    }
  }
  return lines.join('\n');
}
"#;

    let output = run_with_timeout(
        Command::new("osascript")
            .arg("-l")
            .arg("JavaScript")
            .arg("-e")
            .arg(SCRIPT)
            .arg(path),
    )
    .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "macOS OCR failed".to_string()
        } else {
            stderr
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(windows)]
async fn recognize_file(path: &Path) -> Result<String, String> {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$path = __PATH__
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Storage.StorageFile,Windows.Storage,ContentType=WindowsRuntime]
$null = [Windows.Storage.Streams.IRandomAccessStream,Windows.Storage.Streams,ContentType=WindowsRuntime]
$null = [Windows.Graphics.Imaging.BitmapDecoder,Windows.Graphics.Imaging,ContentType=WindowsRuntime]
$null = [Windows.Media.Ocr.OcrEngine,Windows.Media,ContentType=WindowsRuntime]
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
  $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
})[0]
function Await($operation, $resultType) {
  $asTask = $asTaskGeneric.MakeGenericMethod($resultType)
  $task = $asTask.Invoke($null, @($operation))
  $task.Wait(-1) | Out-Null
  $task.Result
}
$file = Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync($path)) ([Windows.Storage.StorageFile])
$stream = Await ($file.OpenAsync([Windows.Storage.FileAccessMode]::Read)) ([Windows.Storage.Streams.IRandomAccessStream])
$decoder = Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
$bitmap = Await ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
if ($null -eq $engine) { throw 'No Windows OCR language pack is installed' }
$result = Await ($engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}
$OutputEncoding = [System.Text.Encoding]::UTF8
$result.Text
"#;

    // PowerShell single-quoted strings only need single quotes doubled.
    let escaped_path = path.to_string_lossy().replace('\'', "''");
    let script = SCRIPT.replace("__PATH__", &format!("'{escaped_path}'"));

    let output = run_with_timeout(
        Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &script,
            ])
            .creation_flags(CREATE_NO_WINDOW),
    )
    .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "Windows OCR failed".to_string()
        } else {
            stderr
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(not(any(target_os = "macos", windows)))]
async fn recognize_file(_path: &Path) -> Result<String, String> {
    Err("Local OCR is not supported on this platform".to_string())
}

async fn run_with_timeout(command: &mut Command) -> Result<std::process::Output, String> {
    command.kill_on_drop(true);
    match tokio::time::timeout(Duration::from_secs(15), command.output()).await {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(e)) => Err(format!("Could not start the OCR engine: {e}")),
        Err(_) => Err("OCR timed out".to_string()),
    }
}

fn temp_path(extension: &str) -> PathBuf {
    std::env::temp_dir().join(format!("truehalo_ocr_{}.{}", uuid::Uuid::new_v4(), extension))
}


#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn recognizes_text_on_macos() {
        const IMAGE: &str = "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";
        let text = recognize(IMAGE, "image/png").await.unwrap();
        assert!(text.contains("TrueHalo"), "unexpected OCR output: {text}");
        assert!(text.contains("123"), "unexpected OCR output: {text}");
    }
}
