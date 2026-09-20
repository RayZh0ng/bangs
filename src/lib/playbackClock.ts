import type { MediaState } from "./native";

/**
 * How far the position may step back before it counts as going somewhere
 * rather than catching up. Dragging the progress bar moves it further.
 */
const CATCH_UP_S = 1;

export interface PlaybackClock {
  anchorAt: number;
  anchorElapsed: number | null;
  revision: number;
  sampleAt?: number;
  sampleElapsed?: number | null;
}

export function trackIdentity(media: MediaState): string {
  return [media.title, media.artist, media.album]
    .map((value) => value.trim().replace(/\s+/gu, " ").toLowerCase())
    .join("\u001f");
}

export function elapsedAt(media: MediaState, now: number, clock: PlaybackClock): number | null {
  if (clock.anchorElapsed == null) return null;
  const elapsed = clock.anchorElapsed + (media.playing ? Math.max(0, now - clock.anchorAt) / 1000 : 0);
  return Math.max(0, media.duration && media.duration > 0 ? Math.min(elapsed, media.duration) : elapsed);
}

/** Ignore stale media samples so focus changes cannot rewind a running clock. */
export function reconcileClock(
  previous: MediaState | null,
  next: MediaState | null,
  clock: PlaybackClock,
  now: number,
  wallNow: number,
): PlaybackClock {
  const changedTrack = previous && next
    ? trackIdentity(previous) !== trackIdentity(next)
    : previous !== next;
  const sourceChanged = !!previous && !!next && previous.sourceId !== next.sourceId;
  const changedPlayback = !!previous && !!next && previous.playing !== next.playing;
  if (!next) return { anchorAt: now, anchorElapsed: null, revision: clock.revision + Number(changedTrack) };

  const valid = next.elapsed != null && Number.isFinite(next.elapsed) && next.elapsed >= 0
    && Number.isFinite(next.elapsedAt) && next.elapsedAt > 0 && next.elapsedAt <= wallNow + 1000;
  const stale = !changedTrack && clock.sampleAt != null
    && (next.elapsedAt < clock.sampleAt
      || (next.elapsedAt === clock.sampleAt && next.elapsed === clock.sampleElapsed));
  const predicted = previous ? elapsedAt(previous, now, clock) : null;
  if (!changedTrack && !changedPlayback && (sourceChanged || stale || !valid)) return clock;

  const sampled = valid && !stale
    ? next.elapsed! + (next.playing ? Math.max(0, wallNow - next.elapsedAt) / 1000 : 0)
    : changedTrack ? null : predicted;
  // A pause reports where the player was when it last looked, a fraction of a
  // second before the sound stopped. The clock on screen has already run past
  // that, and snapping back to it moves the lyric a line and then back again.
  // A step longer than CATCH_UP_S is somewhere the player actually went.
  const behind = predicted != null && sampled != null && sampled < predicted;
  const anchorElapsed =
    behind && predicted! - sampled! <= CATCH_UP_S ? predicted : sampled;
  const jumped = predicted != null && anchorElapsed != null && Math.abs(anchorElapsed - predicted) > 0.75;
  return {
    anchorAt: now,
    anchorElapsed,
    revision: clock.revision + Number(changedTrack || changedPlayback || jumped),
    sampleAt: valid && !stale ? next.elapsedAt : changedTrack ? undefined : clock.sampleAt,
    sampleElapsed: valid && !stale ? next.elapsed : changedTrack ? undefined : clock.sampleElapsed,
  };
}
