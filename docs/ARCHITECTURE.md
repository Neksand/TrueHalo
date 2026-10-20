# TrueHalo Architecture Document

TrueHalo is a fast, responsive AI companion and screen overlay for macOS and Windows. It integrates multiple LLM providers (OpenRouter, Google Gemini, OpenAI), system credential storage and on-demand screen capture. Platform-specific code is quarantined behind small modules, so the frontend and the streaming core are shared.

---

## 1. Directory Structure

```
TrueHalo/
├── src/                                  # React 18 + TypeScript + Tailwind CSS Frontend
│   ├── assets/                           # Branding: Angel Halo icon
│   ├── components/                       # Shared UI components
│   │   └── markdown/                     # MarkdownRenderer (safe GFM & code blocks)
│   ├── features/
│   │   ├── chat/                         # ChatView, message bubbles, streaming cursor
│   │   ├── providers/                    # ProviderSettingsModal, credential management
│   │   └── capture/                      # Screen capture interface (multimodal attachments)
│   ├── lib/
│   │   ├── types.ts                      # Common types (ChatMessage, ModelInfo, ProviderId)
│   │   └── tauri.ts                      # Typed Tauri IPC bridge
│   ├── styles/                           # Tailwind CSS tokens & window-effect rules
│   ├── App.tsx                           # Root orchestrator
│   └── main.tsx                          # DOM mount
├── src-tauri/                            # Rust & Tauri 2 Native Backend
│   ├── src/
│   │   ├── main.rs                       # App binary entrypoint
│   │   ├── lib.rs                        # Library entrypoint & builder initialization
│   │   ├── app_state.rs                  # Active stream registry & cancellation
│   │   ├── ai/                           # AI provider abstraction
│   │   │   ├── provider.rs               # AiProvider trait
│   │   │   ├── types.rs                  # Common provider types
│   │   │   ├── openrouter.rs             # OpenRouter client & SSE stream
│   │   │   ├── gemini.rs                 # Google Gemini API client
│   │   │   └── openai.rs                 # OpenAI API client
│   │   ├── auth/
│   │   │   ├── keychain.rs               # macOS: Security.framework Keychain
│   │   │   └── windows_store.rs          # Windows: Credential Manager (keyring)
│   │   ├── capture/
│   │   │   ├── mod.rs                    # capture orchestration + downscale/JPEG compression
│   │   │   ├── macos.rs                  # macOS: /usr/sbin/screencapture
│   │   │   ├── windows.rs                # Windows: PowerShell + WinForms capture
│   │   │   └── permission.rs             # macOS TCC preflight/request FFI
│   │   └── commands/                     # Tauri typed commands
│   │       ├── ai_commands.rs            # list_models, start_stream, cancel_stream
│   │       ├── auth_commands.rs          # save_api_key, delete_api_key, has_api_key
│   │       ├── capture_commands.rs       # capture_screen, screen permission flow
│   │       └── window_commands.rs        # overlay visibility, vibrancy, stealth mode
│   ├── icons/                            # Icon assets (icns, ico, png 32..512)
│   ├── Cargo.toml                        # Rust dependencies
│   ├── tauri.conf.json                   # Tauri 2 configuration
│   └── tauri.windows.conf.json           # Windows overrides (undecorated window, NSIS)
├── scripts/
│   ├── setup-signing.sh                  # creates the stable local signing identity
│   └── build.sh                          # build + sign + verify + (optional) install
├── docs/
│   └── ARCHITECTURE.md
├── SECURITY.md
└── package.json
```

---

## 2. Core Concepts

### Provider Abstraction (`AiProvider`)
All AI providers implement the `AiProvider` trait:
```rust
#[async_trait]
pub trait AiProvider: Send + Sync {
    fn provider_id(&self) -> &'static str;
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
    async fn stream(
        &self,
        request: CompletionRequest,
        event_tx: mpsc::Sender<StreamEventPayload>,
        cancel_rx: oneshot::Receiver<()>,
    ) -> Result<()>;
}
```

### Cancellation Architecture
Every streaming completion assigns a unique `stream_id` (UUIDv4).
- The `AppState` holds an asynchronous map of `oneshot::Sender<()>` cancel channels.
- When the user presses the **Stop** button (or hits `Escape`), `cancel_stream` drops or signals the cancel channel.
- The provider loop terminates the HTTP stream immediately without lingering connection resources or token waste.

### Zero-Exposure Keychain Storage
- API keys are submitted over Tauri IPC directly into Apple's `Security.framework` via `SecGenericPassword`.
- The frontend only knows whether a key exists (`has_api_key`), keeping secret credentials strictly isolated in native memory.

### Screen Capture Permission (TCC)
- TrueHalo asks macOS for Screen Recording access itself through `CGPreflightScreenCaptureAccess` / `CGRequestScreenCaptureAccess`; it never relies on the permission prompt of an `/usr/sbin/screencapture` subprocess.
- The system dialog is shown at most once per process (guarded by an `AtomicBool`); afterwards the UI opens the exact System Settings pane and offers a restart, because macOS only applies a freshly granted permission after relaunch.
- `capture_screen` never triggers the dialog implicitly: it returns `permission_required` and the frontend shows the permission guide.

### Stable Code Signing & TCC Durability
- macOS keys the Screen Recording grant to the signed code's designated requirement. An ad-hoc signature is identified by its CDHash, which changes on every rebuild, so the grant silently stops matching and macOS asks again.
- `scripts/setup-signing.sh` creates a stable local certificate (`TrueHalo Local Dev`); `scripts/build.sh` signs the whole bundle, so the requirement becomes `identifier "com.truehalo.desktop" and certificate root = H"..."` and the permission survives rebuilds.

### Screenshot Pipeline
- Capture always happens while the overlay is hidden (the application is hidden, the window stays ordered in), so the app can never appear inside its own screenshot.
- The raw PNG is decoded, downscaled to a preset longest edge (1152 / 1568 / 2048 px) and re-encoded as JPEG, cutting payloads from tens of megabytes to a few hundred kilobytes.
- Only the newest screenshot is kept in the request history; older image parts are replaced with a placeholder before streaming.

### Window Transparency & Stealth
- The webview is transparent (`html`, `body`, `#root` are transparent) and a single `.app-backdrop` layer reads the `--window-opacity` CSS variable. The slider maps **1:1 to the alpha channel** (5% = almost invisible, 100% = genuinely solid), so transparency is real transparency rather than a permanent blur.
- Blur is opt-in: `vibrancy` defaults to `false`. When enabled, a native `NSVisualEffectView` (`hudWindow` material) is applied through `window-vibrancy`; when disabled, any stale material is explicitly cleared at startup.
- `NSWindowSharingNone` hides the window from other apps' recordings and screenshots.
- An optional accessory activation policy removes the app from the Dock and ⌘Tab, with a menu bar icon providing Show/Hide, captures, Settings and Quit.

### Window Lifecycle, Hotkey & Geometry
- `Option+Space` toggles the overlay inside the Rust global-shortcut handler (no round-trip through the webview).
- Toggles are debounced (180 ms) so key auto-repeat or duplicate plugin events cannot cause a show/hide flip-flop.
- Hiding never calls `window.hide()`: ordering the **last** window out makes AppKit terminate the process ~1.7 s later (confirmed by logging `-[NSApplication terminate:]` and `applicationWillTerminate`). `overlay::hide` instead hides the **application** (`NSApp hide:`), which keeps the window ordered in — the process survives, the window still disappears from the screen and from screenshots — and `overlay::show` calls `unhide:`, `activateIgnoringOtherApps:` and `makeKeyAndOrderFront:`.
- AppKit may only be used from the main thread: a direct call from a capture worker traps with `EXC_BREAKPOINT` / "Must only be used from the main thread". `overlay::show/hide` marshal work through `run_on_main_thread`, and `overlay::is_visible` answers from an atomic when called off the main thread.
- `NSProcessInfo.disableAutomaticTermination` / `disableSuddenTermination` are called at startup as an extra guard for a menu-bar utility.
- `RunEvent::ExitRequested` with `code: None` (emitted when the last window disappears) is **prevented**: a menu-bar utility must survive losing its window. An explicit quit (tray → Quit, ⌘Q, `app.exit(0)`) still exits.
- If the main window is ever destroyed, `ensure_main_window` recreates it on the next hotkey/tray action.
- The window frame (position + size, physical pixels) is captured from `WindowEvent::Moved/Resized` into atomics and flushed to the `preferences` table by a background thread every 1.2 s (no AppKit access off the main thread), then restored on launch, clamped to an existing monitor.
- Everything is logged to `~/Library/Logs/TrueHalo/truehalo.log` (2 MB rotation), because Finder launches discard stdout.

### Keychain Access Control (no repeated password prompts)
- macOS binds a keychain item's ACL to the exact code signature that created it. Updating an existing item keeps the old ACL, which is why an older build kept triggering the login-password dialog.
- `save_credential` therefore **deletes and re-creates** the item, binding it to the current stable certificate.
- `has_credential` queries **attributes only** (`kSecReturnAttributes`, no `kSecReturnData`), which never touches the protected secret and never raises a prompt — so opening Settings or launching the app is silent.
- Settings → Privacy → *Password prompt when accessing keys* runs `repair_keychain_access`, which reads each existing key once (one system dialog) and rewrites it with the current signature.

