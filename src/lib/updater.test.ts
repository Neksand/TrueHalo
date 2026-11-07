import { afterEach, describe, expect, it, vi } from "vitest";
import { checkForUpdate, isNewer } from "./updater";

describe("isNewer", () => {
  it("compares numeric segments", () => {
    expect(isNewer("0.4.1", "0.4.0")).toBe(true);
    expect(isNewer("v0.5.0", "0.4.9")).toBe(true);
    expect(isNewer("0.4.10", "0.4.9")).toBe(true);
    expect(isNewer("0.4.0", "0.4.0")).toBe(false);
    expect(isNewer("0.3.9", "0.4.0")).toBe(false);
  });
});

describe("checkForUpdate", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it("parses the latest release", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () =>
        new Response(
          JSON.stringify({
            tag_name: "v0.4.1",
            html_url: "https://github.com/Neksand/TrueHalo/releases/tag/v0.4.1",
            body: "Release notes",
          }),
          { status: 200 },
        ),
      ),
    );

    await expect(checkForUpdate("0.4.0")).resolves.toEqual({
      available: true,
      latest: "0.4.1",
      url: "https://github.com/Neksand/TrueHalo/releases/tag/v0.4.1",
    });
  });

  it("rejects on a non-OK response", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => new Response("", { status: 403 })));
    await expect(checkForUpdate("0.4.0")).rejects.toThrow("403");
  });

  it("aborts a hung request", async () => {
    vi.useFakeTimers();
    vi.stubGlobal(
      "fetch",
      vi.fn(
        (_url: string, init: RequestInit) =>
          new Promise((_resolve, reject) => {
            init.signal?.addEventListener("abort", () =>
              reject(new Error("aborted")),
            );
          }),
      ),
    );

    const pending = checkForUpdate("0.4.0");
    const assertion = expect(pending).rejects.toThrow("aborted");
    await vi.advanceTimersByTimeAsync(8000);
    await assertion;
  });
});
