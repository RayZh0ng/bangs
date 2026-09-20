import { create } from "zustand";

import { native, type MediaCommand, type MediaState } from "../lib/native";
import { elapsedAt, reconcileClock, type PlaybackClock } from "../lib/playbackClock";

export { elapsedAt, trackIdentity } from "../lib/playbackClock";
export type { PlaybackClock } from "../lib/playbackClock";

const RECENT_MS = 30 * 60_000;

interface MediaStore {
  media: MediaState | null;
  lastActiveAt: number;
  clock: PlaybackClock;
  update(next: MediaState | null): void;
  send(command: MediaCommand): void;
}

function monotonicNow() {
  return typeof performance !== "undefined" ? performance.now() : Date.now();
}

export const useMedia = create<MediaStore>((set, get) => ({
  media: null,
  lastActiveAt: 0,
  clock: { anchorAt: monotonicNow(), anchorElapsed: null, revision: 0 },

  update(next) {
    const previous = get().media;
    const now = monotonicNow();
    const clock = reconcileClock(previous, next, get().clock, now, Date.now());
    const touched = !!next && (next.playing || !previous || previous.playing);
    set({ media: next, lastActiveAt: touched ? now : get().lastActiveAt, clock });
  },

  send(command) {
    const media = get().media;
    if (media && command.action === "toggle") {
      const now = monotonicNow();
      const elapsed = elapsedAt(media, now, get().clock);
      get().update({ ...media, playing: !media.playing, elapsed, elapsedAt: Date.now() });
    }
    native.media(command).catch((error) => console.warn("media command failed", error));
  },
}));

export function isMediaLive(media: MediaState | null, lastActiveAt: number, now: number) {
  return !!media && (media.playing || now - lastActiveAt < RECENT_MS);
}
