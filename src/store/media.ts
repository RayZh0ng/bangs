import { create } from "zustand";

import { native, type MediaCommand, type MediaState } from "../lib/native";

/**
 * How long a paused track keeps counting as live. Long enough that the notch
 * can still resume what you paused a while ago, short enough that a player
 * left open overnight does not sit in the strip forever.
 */
const RECENT_MS = 30 * 60_000;

/**
 * How far the position may step back before it counts as going somewhere
 * rather than catching up. A player reports where it was when it last looked,
 * which on a pause is a moment before the sound actually stopped; dragging the
 * progress bar moves it further than this.
 */
const CATCH_UP_S = 1;

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
    const now = Date.now();
    // Seeing a track for the first time counts, even paused: at start-up the
    // player may have been sitting paused for hours, and the panel still has
    // to be able to show it — and start it again.
    const touched = !!next && (next.playing || !previous || previous.playing);
    set({ media: settle(previous, next, now), lastActiveAt: touched ? now : get().lastActiveAt });
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

/**
 * Keeps the position from stepping backwards over what is already on screen.
 * Pausing reports where the player was when it last looked, a fraction of a
 * second before the sound stopped, and the lyric would jump back a line and
 * then forward again. A step longer than `CATCH_UP_S` is somewhere the player
 * actually went — a seek, or another track — and is followed.
 */
export function settle(previous: MediaState | null, next: MediaState | null, now: number) {
  if (!next || !previous || !sameTrack(previous, next)) return next;
  const shown = elapsedAt(previous, now);
  const incoming = elapsedAt(next, now);
  if (shown == null || incoming == null) return next;
  const back = shown - incoming;
  if (back <= 0 || back > CATCH_UP_S) return next;
  // Carry on from where the clock had got to, under whatever the player now
  // says about playing or not.
  return { ...next, elapsed: shown, elapsedAt: now };
}

function sameTrack(previous: MediaState, next: MediaState) {
  return (
    previous.sourceId === next.sourceId &&
    previous.title === next.title &&
    previous.artist === next.artist
  );
}

/** Playback position in seconds at `now`, extrapolated while playing. */
export function elapsedAt(media: MediaState, now: number): number | null {
  if (media.elapsed == null) return null;
  const elapsed = media.playing ? media.elapsed + (now - media.elapsedAt) / 1000 : media.elapsed;
  return Math.max(0, media.duration ? Math.min(elapsed, media.duration) : elapsed);
}

export function isMediaLive(media: MediaState | null, lastActiveAt: number, now: number) {
  return !!media && (media.playing || now - lastActiveAt < RECENT_MS);
}
