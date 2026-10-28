import { ModelPricing } from "./types";

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
