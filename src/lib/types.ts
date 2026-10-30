export type ProviderId = "openrouter" | "gemini" | "openai" | "local";

export interface ModelPricing {
  prompt?: string;
  completion?: string;
}

export interface ModelInfo {
  id: string;
  name: string;
  description?: string;
  context_length?: number;
  supports_vision: boolean;
  pricing?: ModelPricing;
}

export interface ContentPart {
  type: "text" | "image_url" | "image_base64";
  text?: string;
  url?: string;
  media_type?: string;
  data?: string;
}

export interface TokenUsage {
  prompt_tokens?: number;
  completion_tokens?: number;
  total_tokens?: number;
  /** Stored with each saved assistant message so costs survive reloads. */
  model?: string;
  provider?: string;
}

export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: ContentPart[];
  timestamp: number;
  isStreaming?: boolean;
  usage?: TokenUsage;
}

export interface CompletionRequest {
  stream_id: string;
  provider: string;
  model: string;
  messages: {
    role: "user" | "assistant" | "system";
    content: ContentPart[];
  }[];
  temperature?: number;
  max_tokens?: number;
}

export type StreamEventPayload =
  | { type: "delta"; text: string }
  | {
      type: "done";
      prompt_tokens?: number;
      completion_tokens?: number;
      total_tokens?: number;
    }
  | { type: "error"; message: string };

/** Options sent to the Rust screen capture command. */
export interface CaptureOptions {
  interactive: boolean;
  show_cursor: boolean;
  compress: boolean;
  max_dimension: number;
  quality: number;
  hide_window: boolean;
  ocr: boolean;
}

export type CaptureResponse =
  | {
      status: "success";
      base64_data: string;
      media_type: string;
      file_size_bytes: number;
      original_size_bytes: number;
      width: number;
      height: number;
      compressed: boolean;
      ocr_text?: string | null;
    }
  | { status: "cancelled" }
  | { status: "permission_required" };

export interface ScreenCaptureStatus {
  granted: boolean;
  prompt_shown: boolean;
  needs_user_action: boolean;
  supported: boolean;
}

/** Compression preset for outgoing screenshots. */
export type CompressionPreset = "quality" | "balanced" | "speed";

/** How captured screenshots are sent to the model. Since every capture keeps
 * both the image and the OCR text, this is only the default for the per-capture
 * Image / Text choice. */
export type TextMode = "auto" | "image" | "text";

/** One-click prompt applied to the current screenshot or recognized text. */
export interface QuickAction {
  id: string;
  label: string;
  prompt: string;
  system?: string;
  custom?: boolean;
}

export interface AppSettings {
  windowOpacity: number;
  vibrancy: boolean;
  hideFromDock: boolean;
  hideFromTaskbar: boolean;
  clickThrough: boolean;
  excludeScreenRecord: boolean;
  alwaysOnTop: boolean;
  compressScreenshots: boolean;
  compressionPreset: CompressionPreset;
  hideWindowOnCapture: boolean;
  keepImagesInHistory: boolean;
  ephemeralHistory: boolean;
  localEndpoint: string;
  /** Tauri accelerator strings, e.g. "Alt+Shift+Digit2". */
  hotkeyToggle: string;
  hotkeyRegion: string;
  hotkeyFullscreen: string;
  textMode: TextMode;
}

/** One attached file inside an assistant profile. */
export interface ProfileFile {
  name: string;
  content: string;
}

/** Named answering context: system instructions plus attached files. */
export interface AssistantProfile {
  id: string;
  name: string;
  system: string;
  files: ProfileFile[];
  /** Skip the size cap for this profile. */
  unlimited: boolean;
  createdAt: number;
}

/** Hard cap for profile context unless the profile is unlimited. */
export const PROFILE_CONTEXT_LIMIT = 200 * 1024;

/** Rough token estimate for a profile (~4 chars per token). */
export function estimateProfileTokens(profile: Pick<AssistantProfile, "system" | "files">): number {
  const chars =
    profile.system.length +
    profile.files.reduce((sum, file) => sum + file.content.length, 0);
  return Math.ceil(chars / 4);
}

export const DEFAULT_SETTINGS: AppSettings = {
  windowOpacity: 0.92,
  // Real transparency by default: vibrancy (blur) is an opt-in "glass" mode.
  vibrancy: false,
  hideFromDock: false,
  hideFromTaskbar: true,
  clickThrough: false,
  excludeScreenRecord: true,
  alwaysOnTop: false,
  compressScreenshots: true,
  compressionPreset: "balanced",
  hideWindowOnCapture: true,
  keepImagesInHistory: true,
  ephemeralHistory: false,
  localEndpoint: "http://localhost:11434/v1",
  hotkeyToggle: "Alt+Space",
  hotkeyRegion: "Alt+Shift+Digit2",
  hotkeyFullscreen: "Alt+Shift+Digit3",
  textMode: "auto",
};

export const COMPRESSION_PRESETS: Record<
  CompressionPreset,
  { label: string; maxDimension: number; quality: number }
> = {
  quality: {
    label: "Quality",
    maxDimension: 2048,
    quality: 88,
  },
  balanced: {
    label: "Balanced",
    maxDimension: 1568,
    quality: 80,
  },
  speed: {
    label: "Speed",
    maxDimension: 1152,
    quality: 60,
  },
};

export interface SessionRecord {
  id: string;
  title: string;
  provider: string;
  model: string;
  created_at: number;
  updated_at: number;
}

export interface UsageTotals {
  prompt_tokens: number;
  completion_tokens: number;
  responses: number;
}

export interface UsageRow {
  provider: string;
  model: string;
  prompt_tokens: number;
  completion_tokens: number;
  responses: number;
  last_used: number;
}

export interface DailyUsage {
  day: string;
  prompt_tokens: number;
  completion_tokens: number;
}

export interface UsageSummary {
  today: UsageTotals;
  all_time: UsageTotals;
  rows: UsageRow[];
  daily: DailyUsage[];
}

export interface MessageRecord {
  id: string;
  session_id: string;
  role: "user" | "assistant" | "system";
  content_json: string;
  timestamp: number;
  usage_json?: string | null;
}
