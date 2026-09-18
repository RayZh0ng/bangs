import { create } from "zustand";

import { native, type MediaCommand, type MediaState } from "../lib/native";

/**
 * How long a paused track keeps counting as live. Long enough that the notch
 * can still resume what you paused a while ago, short enough that a player
 * left open overnight does not sit in the strip forever.
 */
const RECENT_MS = 30 * 60_000;

interface MediaStore {
  media: MediaState | null;
  /** Last time a track was seen playing (or stopped playing). */
  lastActiveAt: number;
  update(next: MediaState | null): void;
  send(command: MediaCommand): void;
}

export const useMedia = create<MediaStore>((set, get) => ({
  media: null,
  lastActiveAt: 0,

  update(next) {
    const previous = get().media;
    // Seeing a track for the first time counts, even paused: at start-up the
    // player may have been sitting paused for hours, and the panel still has
    // to be able to show it — and start it again.
    const touched = !!next && (next.playing || !previous || previous.playing);
    set({ media: next, lastActiveAt: touched ? Date.now() : get().lastActiveAt });
  },

  send(command) {
    const media = get().media;
    // Flip play state right away; the next native update confirms it.
    if (media && command.action === "toggle") {
      const now = Date.now();
      get().update({ ...media, playing: !media.playing, elapsed: elapsedAt(media, now), elapsedAt: now });
    }
    native.media(command).catch((error) => console.warn("media command failed", error));
  },
}));

/** Playback position in seconds at `now`, extrapolated while playing. */
export function elapsedAt(media: MediaState, now: number): number | null {
  if (media.elapsed == null) return null;
  const elapsed = media.playing ? media.elapsed + (now - media.elapsedAt) / 1000 : media.elapsed;
  return Math.max(0, media.duration ? Math.min(elapsed, media.duration) : elapsed);
}

export function isMediaLive(media: MediaState | null, lastActiveAt: number, now: number) {
  return !!media && (media.playing || now - lastActiveAt < RECENT_MS);
}
