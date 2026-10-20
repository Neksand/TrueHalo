# TrueHalo Security Architecture & Threat Model

TrueHalo is built strictly following the principle of least privilege, zero-knowledge credential storage, and strict platform compliance.

---

## 1. Threat Model & Security Posture

| Surface | Defense & Design Decision |
|---|---|
| **API Keys & Credentials** | Stored strictly in the **system credential store** — macOS Keychain (`Security.framework`, `SecGenericPassword`) today, Windows Credential Manager in the Windows port. Keys are **never** persisted to plaintext config files, `.env`, SQLite databases, frontend `localStorage`/`sessionStorage`, or debug logs. |
| **Frontend IPC** | Tauri 2 IPC commands do not return secret keys to the webview. Commands only expose boolean status checks (`has_api_key`) and credential lifecycle triggers (`save_api_key`, `delete_api_key`). |
| **Content Security Policy (CSP)** | Enforces strict CSP: restricts script execution to `'self'`, prevents inline script injections, disables unapproved external navigation. |
| **Markdown / Model Outputs** | Markdown rendering sanitizes HTML tags. Model output text is rendered purely as text nodes and safe markdown components. No `dangerouslySetInnerHTML` is used. |
| **Network Traffic** | Direct outbound HTTPS/TLS connections to official API endpoints (`openrouter.ai`, `googleapis.com`, `api.openai.com`). No telemetry, analytics, or background tracking. |
| **Screen Capture & Privacy** | Screen capture is strictly on-demand. TrueHalo does not run background recording or covert screen polling. Screenshots are captured only after hiding the overlay window, are downscaled/compressed in memory, and are transmitted only when the user explicitly clicks send. Temporary PNG files are deleted immediately after processing. |
| **Screen Recording Permission (TCC)** | Requested through Apple's own CoreGraphics APIs (`CGPreflightScreenCaptureAccess` / `CGRequestScreenCaptureAccess`), never through a spawned helper process. The system dialog is shown at most once per run; everything else goes through the documented System Settings pane. |
| **Code Signing** | The bundle is signed with a stable local identity (`TrueHalo Local Dev`, a self-signed code-signing certificate kept in the user's login keychain) and verified with `codesign --verify --deep --strict` on every build. This keeps the Screen Recording grant valid instead of requiring the user to re-approve each build. |
| **macOS Transparency & Window Privacy** | Uses official AppKit & Tauri window APIs. No undocumented private APIs or surveillance bypasses are used. |

---

## 2. Stored Data

- **System credential store** (macOS Keychain; Windows Credential Manager in the port):
  - Service: `com.truehalo.desktop`
  - Accounts: `provider:openrouter`, `provider:gemini`, `provider:openai`
  - Protection: Protected by macOS user login and system access control.
  - **Access control (ACL)**: the OS binds each item's ACL to the exact code signature. `save_credential` deletes and re-creates the item so it belongs to the current stable certificate, and `has_credential` asks only for item *attributes* (`kSecReturnAttributes`), never the secret — so status checks in Settings cannot raise the login-password dialog. Keys saved by older builds can be re-bound once through Settings → Privacy → "Fix".
- **Application Preferences & History**:
  - Active provider / model choices.
  - Window opacity, vibrancy, stealth and screenshot compression settings.
  - Window position and size (physical pixels), restored on launch.
  - Chat history (local SQLite).
  - No secret tokens are ever placed in the database.

---

## 3. Network Flow Architecture

```
+-------------------------------------------------------------+
|                      React Webview                          |
|  - UI / Markdown Renderer                                    |
|  - No API keys in memory                                    |
+------------------------------+------------------------------+
                               | Typed IPC (invoke/listen)
                               v
+-------------------------------------------------------------+
|                    Tauri 2 Rust Backend                     |
|  - AppState / Stream Management                              |
|  - Queries macOS Keychain for key on request execution       |
|  - Performs direct TLS requests via reqwest                  |
+------------------------------+------------------------------+
                               | TLS / HTTPS (SSE Stream)
                               v
              +----------------------------------+
              | External AI Providers            |
              | - OpenRouter.ai                  |
              | - Google Gemini API              |
              | - OpenAI API                     |
              +----------------------------------+
```
