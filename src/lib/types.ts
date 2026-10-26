export type ProviderId = "openrouter" | "gemini" | "openai";

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

export interface AppSettings {
  windowOpacity: number;
  vibrancy: boolean;
  hideFromDock: boolean;
  excludeScreenRecord: boolean;
  alwaysOnTop: boolean;
  compressScreenshots: boolean;
  compressionPreset: CompressionPreset;
  hideWindowOnCapture: boolean;
  /** Tauri accelerator strings, e.g. "Alt+Shift+Digit2". */
  hotkeyToggle: string;
  hotkeyRegion: string;
  hotkeyFullscreen: string;
}

export const DEFAULT_SETTINGS: AppSettings = {
  windowOpacity: 0.92,
  // Real transparency by default: vibrancy (blur) is an opt-in "glass" mode.
  vibrancy: false,
  hideFromDock: false,
  excludeScreenRecord: true,
  alwaysOnTop: false,
  compressScreenshots: true,
  compressionPreset: "balanced",
  hideWindowOnCapture: true,
  hotkeyToggle: "Alt+Space",
  hotkeyRegion: "Alt+Shift+Digit2",
  hotkeyFullscreen: "Alt+Shift+Digit3",
};

export const COMPRESSION_PRESETS: Record<
  CompressionPreset,
  { label: string; maxDimension: number; quality: number; hint: string }
> = {
  quality: {
    label: "Quality",
    maxDimension: 2048,
    quality: 88,
    hint: "More detail, larger file",
  },
  balanced: {
    label: "Balanced",
    maxDimension: 1568,
    quality: 80,
    hint: "Best for reading text and code",
  },
  speed: {
    label: "Speed",
    maxDimension: 1152,
    quality: 60,
    hint: "Fewest tokens, faster replies",
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

export interface MessageRecord {
  id: string;
  session_id: string;
  role: "user" | "assistant" | "system";
  content_json: string;
  timestamp: number;
}
