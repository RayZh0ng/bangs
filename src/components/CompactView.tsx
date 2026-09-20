import { useEffect, useRef, useState } from "react";

import { t } from "../lib/i18n";
import { centerGap, lyricArtWing, lyricWidth, WING } from "../lib/layout";
import type { LyricWord, MediaState, ScreenInfo } from "../lib/native";
import { CodeIcon, MusicIcon, PauseIcon, ShelfIcon } from "./Icons";
import { KaraokeLine } from "./KaraokeLine";

export interface Activity {
  /** Claude Code sessions waiting for an answer. */
  attention: number;
  media: MediaState | null;
  /** The line being sung right now, when the player has lyrics. */
  lyric: { text: string; translation: string | null; showTranslation: boolean; words?: LyricWord[]; from: number; to: number; elapsed: number; revision: number } | null;
  shelfCount: number;
}

/** Live activity beside the notch, or straight across a screen without one. */
export function CompactView({ activity, screen }: { activity: Activity; screen: ScreenInfo }) {
  const { attention, media, lyric, shelfCount } = activity;
  if (!attention && !media && shelfCount === 0) return null;

  return (
    <div className="compact">
      <div className="compact__wing compact__wing--start" style={{ width: lyric ? lyricArtWing(screen) : WING }}>
        {attention > 0 ? (
          <span className="compact__glyph compact__glyph--attention"><CodeIcon /></span>
        ) : media ? (
          media.artwork ? (
            <img key={media.artwork} className="compact__art" src={media.artwork} alt="" />
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
          <span className="compact__attention">{attention > 1 ? t(`${attention} 个等你`, `${attention} waiting`) : t("等你回复", "Waiting for you")}</span>
        ) : lyric ? (
          <CompactLyric key={lyric.revision} lyric={lyric} fromEnd={screen.hasNotch} />
        ) : media ? (
          media.playing ? <Equalizer /> : <span className="compact__glyph"><PauseIcon /></span>
        ) : (
          <span className="compact__count">{shelfCount}</span>
        )}
      </div>
    </div>
  );
}

/**
 * The line being sung in the strip. The line before it does not disappear the
 * moment the next one starts: it slides up and out from under it.
 */
function CompactLyric({ lyric, fromEnd }: { lyric: NonNullable<Activity["lyric"]>; fromEnd: boolean }) {
  const [leaving, setLeaving] = useState<{ key: number; text: string; translation: string | null } | null>(null);
  const shown = useRef(lyric);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => () => window.clearTimeout(timer.current), []);

  useEffect(() => {
    const previous = shown.current;
    shown.current = lyric;
    if (previous.from === lyric.from) return;
    setLeaving({ key: previous.from, text: previous.text, translation: previous.translation });
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setLeaving(null), LEAVE_MS);
  }, [lyric]);

  return (
    <span className={`lyric-swap${fromEnd ? " lyric-swap--from-end" : ""}`}>
      {leaving && (
        <span key={leaving.key} className="lyric-swap__out">
          <span className="compact__lyric">{leaving.text}</span>
          {lyric.showTranslation && <span className="compact__translation">{leaving.translation ?? ""}</span>}
        </span>
      )}
      <span className="lyric-swap__pair" key={`${lyric.from}:${lyric.revision}`}>
        <KaraokeLine
          className="compact__lyric"
          text={lyric.text}
          words={lyric.words}
          from={lyric.from}
          to={lyric.to}
          elapsed={lyric.elapsed}
        />
        {lyric.showTranslation && <span className="compact__translation">{lyric.translation ?? ""}</span>}
      </span>
    </span>
  );
}

/** Keep in step with the lyric-out animation in styles.css. */
const LEAVE_MS = 420;

function Equalizer() {
  return (
    <span className="equalizer" aria-label={t("正在播放", "Playing")}>
      <i />
      <i />
      <i />
      <i />
    </span>
  );
}
