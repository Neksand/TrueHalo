import { QuickAction } from "./types";

/**
 * Built-in one-click actions. Each action is a system instruction plus a user
 * prompt; both work with a screenshot or with locally recognized text.
 */
export const BUILT_IN_ACTIONS: QuickAction[] = [
  {
    id: "explain",
    label: "Explain",
    system:
      "You are a concise visual assistant. Explain what is on screen in plain language. Use short paragraphs and bullets when they help.",
    prompt:
      "Explain what is shown here. Start with the main idea, then cover the important details.",
  },
  {
    id: "summarize",
    label: "Summarize",
    system:
      "You summarize visual and textual information accurately. Never invent details that are not present in the source.",
    prompt:
      "Summarize the key points in a few short bullets. Include numbers, names, and dates only if they appear in the source.",
  },
  {
    id: "translate",
    label: "Translate",
    system:
      "You are a translator. Preserve meaning, tone, formatting, and code exactly. Do not add commentary.",
    prompt: "Translate all text here into English. If it is already English, translate it into Russian.",
  },
  {
    id: "fix-code",
    label: "Fix code",
    system:
      "You are a senior software engineer. Identify bugs, explain them briefly, and provide corrected code. Do not rewrite working parts unnecessarily.",
    prompt:
      "Review the code shown here. Find bugs or errors, explain the cause, and show the corrected code.",
  },
  {
    id: "extract-text",
    label: "Extract text",
    system:
      "You extract text verbatim. Preserve line breaks, indentation, punctuation, and code. Do not summarize.",
    prompt:
      "Extract all text from this image exactly as written. Preserve code indentation and line breaks.",
  },
  {
    id: "whats-wrong",
    label: "What's wrong?",
    system:
      "You diagnose problems from screenshots: errors, broken layouts, failing tests, or confusing UI. Be specific and practical.",
    prompt:
      "What is wrong here? Identify the problem, explain why it happens, and give the next concrete step to fix it.",
  },
];

export const CUSTOM_ACTIONS_PREF = "custom_quick_actions";

export function parseCustomActions(raw: string | null): QuickAction[] {
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter(
        (item): item is QuickAction =>
          item &&
          typeof item.id === "string" &&
          typeof item.label === "string" &&
          typeof item.prompt === "string",
      )
      .map((item) => ({ ...item, custom: true }));
  } catch {
    return [];
  }
}
