import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import {
  CaptureOptions,
  CaptureResponse,
  CompletionRequest,
  MessageRecord,
  ModelInfo,
  ProviderId,
  ScreenCaptureStatus,
  SessionRecord,
  StreamEventPayload,
  TokenUsage,
} from "./types";

export const tauriApi = {
  // Keychain auth
  async hasApiKey(provider: ProviderId): Promise<boolean> {
    try {
      return await invoke<boolean>("has_api_key", { provider });
    } catch {
      return false;
    }
  },

  async repairKeychainAccess(): Promise<{
    checked: number;
    repaired: number;
    failed: string[];
    has_any_key: boolean;
  }> {
    return await invoke("repair_keychain_access");
  },

  async saveApiKey(provider: ProviderId, key: string): Promise<void> {
    await invoke("save_api_key", { provider, key });
  },

  async deleteApiKey(provider: ProviderId): Promise<void> {
    await invoke("delete_api_key", { provider });
  },

  // AI Operations
  async listModels(provider: ProviderId): Promise<ModelInfo[]> {
    return await invoke<ModelInfo[]>("list_models", { provider });
  },

  async startStream(request: CompletionRequest): Promise<void> {
    await invoke("start_stream", { request });
  },

  async cancelStream(streamId: string): Promise<boolean> {
    return await invoke<boolean>("cancel_stream", { streamId });
  },

  // Listen to streaming events
  async listenStream(
    streamId: string,
    onDelta: (text: string) => void,
    onDone: (usage?: TokenUsage) => void,
    onError: (err: string) => void
  ): Promise<UnlistenFn> {
    const channel = `stream:${streamId}`;
    return await listen<StreamEventPayload>(channel, (event) => {
      const payload = event.payload;
      if (payload.type === "delta") {
        onDelta(payload.text);
      } else if (payload.type === "done") {
        onDone(payload);
      } else if (payload.type === "error") {
        onError(payload.message);
      }
    });
  },

  // Screen Capture
  async captureScreen(options: CaptureOptions): Promise<CaptureResponse> {
    return await invoke<CaptureResponse>("capture_screen", { options });
  },

  async screenCaptureStatus(): Promise<ScreenCaptureStatus> {
    return await invoke<ScreenCaptureStatus>("screen_capture_status");
  },

  async requestScreenCapturePermission(): Promise<ScreenCaptureStatus> {
    return await invoke<ScreenCaptureStatus>("request_screen_capture_permission");
  },

  async openScreenCaptureSettings(): Promise<void> {
    await invoke("open_screen_capture_settings");
  },

  async restartApp(): Promise<void> {
    await invoke("restart_app");
  },

  // SQLite Storage
  async listSessions(): Promise<SessionRecord[]> {
    return await invoke<SessionRecord[]>("list_sessions");
  },

  async createSession(
    id: string,
    title: string,
    provider: string,
    model: string
  ): Promise<SessionRecord> {
    return await invoke<SessionRecord>("create_session", { id, title, provider, model });
  },

  async getSessionMessages(sessionId: string): Promise<MessageRecord[]> {
    return await invoke<MessageRecord[]>("get_session_messages", { sessionId });
  },

  async saveMessage(
    id: string,
    sessionId: string,
    role: string,
    contentJson: string
  ): Promise<void> {
    await invoke("save_message", { id, sessionId, role, contentJson });
  },

  async deleteSession(sessionId: string): Promise<void> {
    await invoke("delete_session", { sessionId });
  },

  async clearAllHistory(): Promise<void> {
    await invoke("clear_all_history");
  },

  async setPreference(key: string, value: string): Promise<void> {
    await invoke("set_preference", { key, value });
  },

  async getPreference(key: string): Promise<string | null> {
    return await invoke<string | null>("get_preference", { key });
  },

  // Window Management
  async toggleOverlay(): Promise<void> {
    await invoke("toggle_overlay");
  },

  async quickHide(): Promise<void> {
    await invoke("quick_hide");
  },

  async showWindow(): Promise<void> {
    await invoke("show_window");
  },

  async setAlwaysOnTop(alwaysOnTop: boolean): Promise<void> {
    await invoke("set_always_on_top", { alwaysOnTop });
  },

  async setVibrancy(enabled: boolean): Promise<boolean> {
    return await invoke<boolean>("set_vibrancy", { enabled });
  },

  async setHideFromDock(hide: boolean): Promise<boolean> {
    return await invoke<boolean>("set_hide_from_dock", { hide });
  },

  async setScreenRecordingProtection(exclude: boolean): Promise<boolean> {
    return await invoke<boolean>("set_screen_recording_protection", { exclude });
  },

  // Hotkey events
  async onHotkey(event: string, callback: () => void): Promise<UnlistenFn> {
    return await listen(event, () => callback());
  },
};
