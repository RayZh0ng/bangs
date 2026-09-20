import type { ScreenInfo, Settings } from "./native";

export type Mode = "compact" | "expanded" | "drop" | "success";

export interface Size {
  width: number;
  height: number;
}

/** Transparent host window size; keep in sync with src-tauri/src/geometry.rs. */
export const WINDOW: Size = { width: 640, height: 280 };

const VIRTUAL_NOTCH_WIDTH = 190;
/** Height of a drawn strip where there is no menu bar to match (Windows). */
const FALLBACK_HEIGHT = 26;
/** Extra width on each side of the notch for live activity in compact mode. */
export const WING = 76;
/** The right wing grows when it carries a lyric line instead of a glyph. */
export const LYRIC_WING = 190;
/** Without a cutout in the way, the lyric gets the whole strip. */
const LYRIC_STRIP = 300;
/** Height of the compact original/translation pair. */
const LYRIC_HEIGHT = 40;
const HANDLE: Size = { width: 130, height: 7 };
const EXPANDED = { width: 600, body: 184 };
const DROP = { width: 420, body: 92 };

/** The resting notch: the hardware cutout, or a virtual one sized like it. */
export function baseNotch(screen: ScreenInfo): Size {
  if (screen.hasNotch) {
    return { width: screen.notchWidth, height: screen.notchHeight };
  }
  const height = screen.menuBarHeight >= 22 ? screen.menuBarHeight : FALLBACK_HEIGHT;
  return { width: VIRTUAL_NOTCH_WIDTH, height };
}

/**
 * Dead space in the middle of the compact strip. A MacBook's cutout has to be
 * flowed around; every other screen — an external display, a Windows one — is
 * flat there, so the content runs straight across instead.
 */
export function centerGap(screen: ScreenInfo): number {
  return screen.hasNotch ? screen.notchWidth : 0;
}

/** Room for the lyric line, which is wider when it needs no cutout beside it. */
export function lyricWidth(screen: ScreenInfo): number {
  return screen.hasNotch ? LYRIC_WING : LYRIC_STRIP;
}

/**
 * The wing carrying the artwork while a lyric is showing. Beside a cutout it
 * matches the other wing; on a flat strip it only has to hold the artwork, and
 * a narrower one keeps the lyric next to the cover instead of adrift.
 */
export function lyricArtWing(screen: ScreenInfo): number {
  return screen.hasNotch ? WING : 50;
}

export function notchSize(
  mode: Mode,
  screen: ScreenInfo,
  settings: Settings,
  hasActivity: boolean,
  lyric: boolean,
  lyricTranslation: boolean,
): Size {
  const base = baseNotch(screen);
  switch (mode) {
    case "expanded":
      return { width: Math.max(EXPANDED.width, base.width + 280), height: base.height + EXPANDED.body };
    case "drop":
    case "success":
      return { width: Math.max(DROP.width, base.width + 160), height: base.height + DROP.body };
    case "compact": {
      const gap = centerGap(screen);
      // Translated lyrics need two readable rows. Original-only lyrics fit the
      // regular compact height.
      if (lyric) {
        const height = lyricTranslation ? Math.max(base.height, LYRIC_HEIGHT) : base.height;
        return { width: gap + lyricArtWing(screen) + lyricWidth(screen), height };
      }
      if (hasActivity) return { width: Math.max(base.width, gap + WING * 2), height: base.height };
      if (settings.idleHandle && !screen.hasNotch) return HANDLE;
      return base;
    }
  }
}

/** Interactive area for a visual size; thin shapes get a more forgiving target. */
export function hitRect(size: Size): Size {
  return { width: Math.max(size.width, 160), height: Math.max(size.height, 12) };
}

export function cornerRadii(mode: Mode, size: Size): { ear: number; bottom: number } {
  switch (mode) {
    case "compact":
      return { ear: Math.min(6, size.height / 2), bottom: Math.min(12, size.height / 2) };
    case "expanded":
      return { ear: 12, bottom: 24 };
    case "drop":
    case "success":
      return { ear: 12, bottom: 28 };
  }
}
