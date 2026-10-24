/**
 * Human-readable global shortcut labels.
 *
 * The backend registers `Alt+Space` / `Alt+Shift+2` / `Alt+Shift+3` on every
 * platform (Option is the macOS name for Alt). The UI should show the same
 * glyphs the user sees on their keyboard, so labels are platform-aware.
 */

export const IS_WINDOWS =
  typeof navigator !== "undefined" && /Windows/i.test(navigator.userAgent);

const isWindows = IS_WINDOWS;

export const HOTKEYS = {
  /** Compact form for inline hints: "⌥Space" / "Alt+Space". */
  toggleCompact: isWindows ? "Alt+Space" : "⌥Space",
  /** Spaced form for <kbd> elements: "⌥ Space" / "Alt+Space". */
  toggle: isWindows ? "Alt+Space" : "⌥ Space",
  region: isWindows ? "Alt+Shift+2" : "⌥⇧2",
  fullscreen: isWindows ? "Alt+Shift+3" : "⌥⇧3",
} as const;
