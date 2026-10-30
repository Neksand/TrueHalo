import { AssistantProfile } from "./types";

export const PROFILES_PREF = "assistant_profiles";
export const ACTIVE_PROFILE_PREF = "active_profile_id";

export function parseProfiles(raw: string | null): AssistantProfile[] {
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter(
        (item): item is AssistantProfile =>
          item &&
          typeof item.id === "string" &&
          typeof item.name === "string" &&
          typeof item.system === "string" &&
          Array.isArray(item.files),
      )
      .map((item) => ({
        id: item.id,
        name: item.name.slice(0, 48),
        system: item.system,
        files: item.files
          .filter(
            (file: unknown): file is { name: string; content: string } =>
              !!file &&
              typeof (file as { name?: unknown }).name === "string" &&
              typeof (file as { content?: unknown }).content === "string",
          )
          .map((file: { name: string; content: string }) => ({
            name: file.name.slice(0, 128),
            content: file.content,
          })),
        unlimited: item.unlimited === true,
        createdAt: typeof item.createdAt === "number" ? item.createdAt : Date.now(),
      }));
  } catch {
    return [];
  }
}

/** The exact text injected as the system message for one request. */
export function profileContextText(profile: AssistantProfile): string {
  const files = profile.files
    .map((file) => `--- ${file.name} ---\n${file.content}`)
    .join("\n\n");
  return [profile.system.trim(), files.trim()].filter(Boolean).join("\n\n");
}
