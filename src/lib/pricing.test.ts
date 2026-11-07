import { describe, expect, it } from "vitest";
import {
  costForTokens,
  estimateRequestTokens,
  formatUsd,
  pricePerToken,
  trimToBudget,
} from "./pricing";

describe("formatUsd", () => {
  it("formats known and unknown prices", () => {
    expect(formatUsd(null)).toBe("—");
    expect(formatUsd(0)).toBe("$0.00");
    expect(formatUsd(0.002)).toBe("$0.00200");
    expect(formatUsd(1.23456)).toBe("$1.2346");
  });
});

describe("pricePerToken", () => {
  it("parses finite numbers only", () => {
    expect(pricePerToken("0.000003")).toBe(0.000003);
    expect(pricePerToken(undefined)).toBeNull();
    expect(pricePerToken("nope")).toBeNull();
  });
});

describe("costForTokens", () => {
  it("adds prompt and completion cost", () => {
    expect(
      costForTokens(1000, 500, { prompt: "0.000001", completion: "0.000002" }),
    ).toBeCloseTo(0.002);
  });

  it("returns null when pricing is incomplete", () => {
    expect(costForTokens(1000, 500, { prompt: "0.000001" })).toBeNull();
    expect(costForTokens(1000, 500, null)).toBeNull();
  });
});

describe("estimateRequestTokens", () => {
  it("counts text at four chars per token and images flat", () => {
    expect(
      estimateRequestTokens([
        { content: [{ type: "text", text: "12345678" }] },
        { content: [{ type: "image_base64", data: "abc" }] },
      ]),
    ).toBe(2 + 1500);
  });
});

describe("trimToBudget", () => {
  const text = (role: string, chars: number) => ({
    role,
    content: [{ type: "text" as const, text: "x".repeat(chars) }],
  });

  it("drops the oldest turns until the request fits", () => {
    const messages = [
      text("user", 400),
      text("assistant", 400),
      text("user", 400),
      text("user", 400),
    ];

    const { messages: kept, trimmed } = trimToBudget(messages, 250);

    expect(trimmed).toBe(2);
    expect(kept).toHaveLength(2);
    expect(kept[kept.length - 1]).toBe(messages[messages.length - 1]);
  });

  it("always keeps system instructions and the newest turn", () => {
    const messages = [
      text("system", 400),
      text("user", 400),
      text("user", 400),
    ];

    const { messages: kept, trimmed } = trimToBudget(messages, 250);

    expect(trimmed).toBe(1);
    expect(kept.map((m) => m.role)).toEqual(["system", "user"]);
    expect(kept[kept.length - 1]).toBe(messages[messages.length - 1]);
  });

  it("leaves everything alone when it already fits", () => {
    const messages = [text("user", 40)];
    expect(trimToBudget(messages, 1000)).toEqual({ messages, trimmed: 0 });
    expect(trimToBudget(messages, 0)).toEqual({ messages, trimmed: 0 });
  });
});
