import { WING, type Size } from "../lib/layout";
import { countdown } from "../lib/format";
import type { MediaState } from "../lib/native";
import type { TimerKind } from "../store/timer";
import { CodeIcon, MusicIcon, PauseIcon, ShelfIcon, TimerIcon } from "./Icons";

export interface Activity {
  /** Claude Code sessions waiting for an answer. */
  attention: number;
  media: MediaState | null;
  timer: { kind: TimerKind; remaining: number; paused: boolean } | null;
  shelfCount: number;
}

/** Live activity on both sides of the (hardware or virtual) notch. */
export function CompactView({ activity, base }: { activity: Activity; base: Size }) {
  const { attention, media, timer, shelfCount } = activity;
  if (!attention && !media && !timer && shelfCount === 0) return null;

  return (
    <div className="compact">
      <div className="compact__wing compact__wing--start" style={{ width: WING }}>
        {attention > 0 ? (
          <span className="compact__glyph compact__glyph--attention"><CodeIcon /></span>
        ) : media ? (
          media.artwork ? (
            <img className="compact__art" src={media.artwork} alt="" />
          ) : (
            <span className="compact__glyph compact__glyph--music"><MusicIcon /></span>
          )
        ) : timer ? (
          <span className={`compact__glyph compact__glyph--${timer.kind}`}><TimerIcon /></span>
        ) : (
          <span className="compact__glyph"><ShelfIcon /></span>
        )}
      </div>

      <div style={{ width: base.width, flex: "none" }} />

      <div className="compact__wing compact__wing--end" style={{ width: WING }}>
        {attention > 0 ? (
          <span className="compact__attention">{attention > 1 ? `${attention} 个等你` : "等你回复"}</span>
        ) : timer ? (
          <span className={`compact__time compact__time--${timer.kind}${timer.paused ? " is-paused" : ""}`}>
            {countdown(timer.remaining)}
          </span>
        ) : media ? (
          media.playing ? <Equalizer /> : <span className="compact__glyph"><PauseIcon /></span>
        ) : (
          <span className="compact__count">{shelfCount}</span>
        )}
      </div>
    </div>
  );
}

function Equalizer() {
  return (
    <span className="equalizer" aria-label="正在播放">
      <i />
      <i />
      <i />
      <i />
    </span>
  );
}
