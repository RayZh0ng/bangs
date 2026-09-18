import type { MouseEvent } from "react";

import { clock } from "../lib/format";
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
  const send = useMedia((s) => s.send);
  const lines = useLyrics((s) => s.lines);
  // The sweep over the current line needs a faster clock than the progress bar.
  const now = useNow(current?.playing ? (lines.length ? 80 : 250) : 1000, !!current);

  // The same rule the compact strip uses, so the two never disagree about
  // whether a long-paused track still counts as playing.
  const media = isMediaLive(current, lastActiveAt, now) ? current : null;

  if (!media) {
    return <Empty icon={<MusicIcon />} title="没有正在播放的音乐" hint="在任意播放器里开始播放，就会显示在这里" />;
  }

  const elapsed = elapsedAt(media, now);
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
        {media.artwork ? <img src={media.artwork} alt="" /> : <MusicIcon width={32} height={32} />}
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
            <button className="control" onClick={() => send({ action: "previous" })} title="上一首">
              <PreviousIcon width={18} height={18} />
            </button>
            <button className="control control--main" onClick={() => send({ action: "toggle" })} title={media.playing ? "暂停" : "播放"}>
              {media.playing ? <PauseIcon width={18} height={18} /> : <PlayIcon width={18} height={18} />}
            </button>
            <button className="control" onClick={() => send({ action: "next" })} title="下一首">
              <NextIcon width={18} height={18} />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

/** The line being sung, with context above and below it. */
function LyricView({ lines, elapsed }: { lines: LyricLine[]; elapsed: number }) {
  const index = lineAt(lines, elapsed);
  const line = lines[index];

  return (
    <div className="lyrics">
      <div className="lyrics__line">{lines[index - 1]?.text ?? ""}</div>
      <div className="lyrics__line is-current">
        {line ? (
          <KaraokeLine
            text={line.text}
            words={line.words}
            from={line.at}
            to={lines[index + 1]?.at ?? line.at + 6}
            elapsed={elapsed}
          />
        ) : (
          ""
        )}
      </div>
      {/* The translation is more useful than the next line when there is one. */}
      <div className="lyrics__line">{line?.translation ?? lines[index + 1]?.text ?? ""}</div>
    </div>
  );
}
