import { ContentPart, ModelPricing } from "./types";

/** `—` means the provider did not report a price for this model. */
export const formatUsd = (value: number | null): string => {
  if (value === null) return "—";
  if (value === 0) return "$0.00";
  if (value < 0.01) return `$${value.toFixed(5)}`;
  return `$${value.toFixed(4)}`;
};

export const pricePerToken = (value?: string): number | null => {
  if (!value) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
};

/** Cost of one response at the provider's per-token list price. */
export const costForTokens = (
  promptTokens: number,
  completionTokens: number,
  pricing?: ModelPricing | null,
): number | null => {
  if (!pricing) return null;
  const promptPrice = pricePerToken(pricing.prompt);
  const completionPrice = pricePerToken(pricing.completion);
  if (promptPrice === null || completionPrice === null) return null;
  return promptTokens * promptPrice + completionTokens * completionPrice;
};

/**
 * Rough token estimate for one outgoing request. Text counts at ~4 chars per
 * token; each image counts flat (providers bill images by size, so this is
 * deliberately conservative). Good enough for a budget trim, never for billing.
 */
export const IMAGE_TOKEN_ESTIMATE = 1500;

export function estimateRequestTokens(
  messages: Array<{ content: ContentPart[] }>,
): number {
  let total = 0;
  for (const message of messages) {
    for (const part of message.content) {
      if (part.type === "text") {
        total += Math.ceil((part.text ?? "").length / 4);
      } else if (part.type === "image_base64" && part.data) {
        total += IMAGE_TOKEN_ESTIMATE;
      }
    }
  }
  return total;
}

/**
 * Drop the oldest turns until the request fits the budget. System messages
 * and the newest user turn always survive; images were already thinned by
 * `stripOlderImages` upstream.
 */
export function trimToBudget<T extends { role: string; content: ContentPart[] }>(
  messages: T[],
  budget: number,
): { messages: T[]; trimmed: number } {
  if (budget <= 0 || estimateRequestTokens(messages) <= budget) {
    return { messages, trimmed: 0 };
  }
  let trimmed = 0;
  let kept = messages;
  while (kept.length > 1 && estimateRequestTokens(kept) > budget) {
    const dropIndex = kept.findIndex(
      (m, i) => m.role !== "system" && i < kept.length - 1,
    );
    if (dropIndex === -1) break;
    kept = [...kept.slice(0, dropIndex), ...kept.slice(dropIndex + 1)];
    trimmed += 1;
  }
  return { messages: kept, trimmed };
}
