import { useMemo, type MouseEvent } from "react";

import { clock } from "../lib/format";
import { t } from "../lib/i18n";
import type { LyricLine } from "../lib/native";
import { useNow } from "../lib/useNow";
import { lineAt, useLyrics } from "../store/lyrics";
import { elapsedAt, isMediaLive, useMedia } from "../store/media";
import { Empty } from "./Empty";
import { KaraokeLine } from "./KaraokeLine";
import { MusicIcon, NextIcon, PauseIcon, PlayIcon, PreviousIcon } from "./Icons";

export function MusicPanel() {
  const current = useMedia((s) => s.media);
  const lastActiveAt = useMedia((s) => s.lastActiveAt);
  const mediaClock = useMedia((s) => s.clock);
  const send = useMedia((s) => s.send);
  const lines = useLyrics((s) => s.lines);
  // The sweep over the current line needs a faster clock than the progress bar.
  const now = useNow(current?.playing ? (lines.length ? 80 : 250) : 1000, !!current, true);

  // The same rule the compact strip uses, so the two never disagree about
  // whether a long-paused track still counts as playing.
  const media = isMediaLive(current, lastActiveAt, now) ? current : null;

  if (!media) {
    return <Empty icon={<MusicIcon />} title={t("没有正在播放的音乐", "Nothing is playing")}
        hint={t("在任意播放器里开始播放，就会显示在这里", "Start something in any player and it shows up here")} />;
  }

  const elapsed = elapsedAt(media, now, mediaClock);
  const duration = media.duration;
  const progress = duration && elapsed != null ? elapsed / duration : null;
  const hasLyrics = lines.length > 0;

  const seek = (event: MouseEvent<HTMLDivElement>) => {
    if (!duration) return;
    const rect = event.currentTarget.getBoundingClientRect();
    const ratio = Math.min(1, Math.max(0, (event.clientX - rect.left) / rect.width));
    send({ action: "seek", position: ratio * duration });
  };

  return (
    <div className="music">
      <div className="music__art">
        {media.artwork ? <img key={media.artwork} src={media.artwork} alt="" /> : <MusicIcon width={32} height={32} />}
      </div>

      <div className="music__main">
        <div>
          <div className="music__title" title={media.title}>{media.title}</div>
          {/* With lyrics the artist moves to the footer to make room. */}
          {!hasLyrics && <div className="music__artist">{media.artist || media.album || " "}</div>}
        </div>

        {hasLyrics && <LyricView lines={lines} elapsed={elapsed ?? 0} />}

        {progress != null && elapsed != null && duration ? (
          <div className="progress" onClick={seek}>
            <div className="progress__track">
              <div className="progress__fill" style={{ width: `${progress * 100}%` }} />
            </div>
            <div className="progress__times">
              <span>{clock(elapsed)}</span>
              <span>-{clock(duration - elapsed)}</span>
            </div>
          </div>
        ) : (
          <div className="progress progress--empty" />
        )}

        <div className="music__footer">
          <span className="music__app">
            {hasLyrics && media.artist ? `${media.artist} · ${media.appName}` : media.appName}
          </span>
          <div className="controls">
            <button className="control" onClick={() => send({ action: "previous" })} title={t("上一首", "Previous")}>
              <PreviousIcon width={18} height={18} />
            </button>
            <button className="control control--main" onClick={() => send({ action: "toggle" })} title={media.playing ? t("暂停", "Pause") : t("播放", "Play")}>
              {media.playing ? <PauseIcon width={18} height={18} /> : <PlayIcon width={18} height={18} />}
            </button>
            <button className="control" onClick={() => send({ action: "next" })} title={t("下一首", "Next")}>
              <NextIcon width={18} height={18} />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

/** Height of one row; rowOfLine counts original AND translation rows. */
const ROW = 20;

interface Row {
  /** Index of the line this row belongs to. */
  line: number;
  text: string;
  translation: boolean;
}

/** Always reserves two rows per lyric so translation cannot shift the original. */
function rowsOf(lines: LyricLine[]): { rows: Row[]; rowOfLine: number[] } {
  const rows: Row[] = [];
  const rowOfLine: number[] = [];
  lines.forEach((line, index) => {
    rowOfLine[index] = rows.length;
    rows.push({ line: index, text: line.text, translation: false });
    rows.push({ line: index, text: line.translation ?? "", translation: true });
  });
  return { rows, rowOfLine };
}

/**
 * The lyric as a strip that scrolls, so the line being sung slides into the
 * middle instead of the two-line pair swapping its text at once.
 */
export function LyricView({ lines, elapsed }: { lines: LyricLine[]; elapsed: number }) {
  const { rows, rowOfLine } = useMemo(() => rowsOf(lines), [lines]);
  const index = lineAt(lines, elapsed);
  const line = lines[index];
  // Each lyric pair occupies two rows; keep the current original in the middle.
  const rowOffset = index < 0 ? 0 : rowOfLine[index];

  return (
    <div className="lyrics">
      <div className="lyrics__scroll" style={{ transform: `translateY(${-rowOffset * ROW}px)` }}>
        {rows.map((row, position) => {
          const current = row.line === index;
          return (
            <div
              key={position}
              className={`lyrics__line${current ? " is-current" : ""}${row.translation ? " lyrics__line--sub" : ""}`}
            >
              {current && line && !row.translation ? (
                <KaraokeLine
                  text={line.text}
                  words={line.words}
                  from={line.at}
                  to={lines[index + 1]?.at ?? line.at + 6}
                  elapsed={elapsed}
                />
              ) : row.text}
            </div>
          );
        })}
      </div>
    </div>
  );
}
