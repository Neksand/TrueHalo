/**
 * Human-readable global shortcut labels.
 *
 * The defaults are `Alt+Space` / `Alt+Shift+Digit2` / `Alt+Shift+Digit3` on
 * every platform (Option is the macOS name for Alt), and users can change them
 * in Settings. The UI shows the glyphs the user sees on their keyboard, so
 * labels are platform-aware.
 */

export const IS_WINDOWS =
  typeof navigator !== "undefined" && /Windows/i.test(navigator.userAgent);

const isWindows = IS_WINDOWS;

/**
 * Format a stored Tauri accelerator for display.
 *
 * `Alt+Shift+Digit2` → `⌥ ⇧ 2` on macOS, `Alt+Shift+2` on Windows. The
 * compact form drops separators so it fits inline hints.
 */
export function formatHotkey(accelerator: string, compact = false): string {
  const parts = accelerator
    .split("+")
    .map((part) => keyLabel(part.trim()))
    .filter(Boolean);

  if (!isWindows) {
    const glyphs = parts.map((part) => {
      switch (part) {
        case "Alt":
          return "⌥";
        case "Shift":
          return "⇧";
        case "Control":
        case "Ctrl":
          return "⌃";
        case "Super":
        case "Command":
        case "Cmd":
        case "CommandOrControl":
          return "⌘";
        default:
          return part;
      }
    });
    return glyphs.join(compact ? "" : " ");
  }

  return parts
    .map((part) => (part === "CommandOrControl" ? "Ctrl" : part))
    .join("+");
}

function keyLabel(part: string): string {
  if (part.startsWith("Digit")) return part.slice("Digit".length);
  if (part.startsWith("Key")) return part.slice("Key".length);
  return part;
}
