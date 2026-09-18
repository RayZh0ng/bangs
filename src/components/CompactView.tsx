import { centerGap, lyricWidth, WING } from "../lib/layout";
import type { LyricWord, MediaState, ScreenInfo } from "../lib/native";
import { CodeIcon, MusicIcon, PauseIcon, ShelfIcon } from "./Icons";
import { KaraokeLine } from "./KaraokeLine";

export interface Activity {
  /** Claude Code sessions waiting for an answer. */
  attention: number;
  media: MediaState | null;
  /** The line being sung right now, when the player has lyrics. */
  lyric: { text: string; words?: LyricWord[]; from: number; to: number; elapsed: number } | null;
  shelfCount: number;
}

/** Live activity beside the notch, or straight across a screen without one. */
export function CompactView({ activity, screen }: { activity: Activity; screen: ScreenInfo }) {
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

      <div style={{ width: centerGap(screen), flex: "none" }} />

      <div
        className="compact__wing compact__wing--end"
        style={{
          width: lyric ? lyricWidth(screen) : WING,
          // Without a cutout the lyric reads on from the artwork; beside one it
          // hangs off the right edge instead.
          justifyContent: lyric && !screen.hasNotch ? "flex-start" : undefined,
        }}
      >
        {attention > 0 ? (
          <span className="compact__attention">{attention > 1 ? `${attention} 个等你` : "等你回复"}</span>
        ) : lyric ? (
          <KaraokeLine key={lyric.from} className="compact__lyric" {...lyric} />
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
