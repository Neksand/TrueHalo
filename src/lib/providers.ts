import { ProviderId } from "./types";

/** One entry per AI provider; shared by Settings and the setup wizard. */
export interface ProviderInfo {
  id: ProviderId;
  name: string;
  defaultModel: string;
  /** Page where the user creates an API key. */
  keyUrl: string;
  blurb: string;
  recommended?: boolean;
}

export const PROVIDERS: ProviderInfo[] = [
  {
    id: "openrouter",
    name: "OpenRouter",
    defaultModel: "minimax/minimax-m3.1-flash:free",
    keyUrl: "https://openrouter.ai/settings/keys",
    blurb: "One key for hundreds of models, including free ones.",
    recommended: true,
  },
  {
    id: "gemini",
    name: "Google Gemini",
    defaultModel: "gemini-1.5-flash",
    keyUrl: "https://aistudio.google.com/app/apikey",
    blurb: "Fast multimodal models from Google AI Studio.",
  },
  {
    id: "openai",
    name: "OpenAI (ChatGPT)",
    defaultModel: "gpt-4o",
    keyUrl: "https://platform.openai.com/api-keys",
    blurb: "GPT-4o and the rest of the OpenAI family.",
  },
  {
    id: "local",
    name: "Local server",
    defaultModel: "",
    keyUrl: "",
    blurb: "Ollama, LM Studio or any OpenAI-compatible server. No key needed.",
  },
];
