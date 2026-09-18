import { LYRIC_WING, WING, type Size } from "../lib/layout";
import type { MediaState } from "../lib/native";
import { CodeIcon, MusicIcon, PauseIcon, ShelfIcon } from "./Icons";

export interface Activity {
  /** Claude Code sessions waiting for an answer. */
  attention: number;
  media: MediaState | null;
  /** The line being sung right now, when the player has lyrics. */
  lyric: string | null;
  shelfCount: number;
}

/** Live activity on both sides of the (hardware or virtual) notch. */
export function CompactView({ activity, base }: { activity: Activity; base: Size }) {
  const { attention, media, lyric, shelfCount } = activity;
  if (!attention && !media && shelfCount === 0) return null;

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
        ) : (
          <span className="compact__glyph"><ShelfIcon /></span>
        )}
      </div>

      <div style={{ width: base.width, flex: "none" }} />

      <div
        className="compact__wing compact__wing--end"
        style={{ width: lyric ? LYRIC_WING : WING }}
      >
        {attention > 0 ? (
          <span className="compact__attention">{attention > 1 ? `${attention} 个等你` : "等你回复"}</span>
        ) : lyric ? (
          <span key={lyric} className="compact__lyric">{lyric}</span>
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
