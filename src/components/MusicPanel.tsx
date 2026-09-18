import type { MouseEvent } from "react";

import { clock } from "../lib/format";
import { useNow } from "../lib/useNow";
import { elapsedAt, useMedia } from "../store/media";
import { Empty } from "./Empty";
import { MusicIcon, NextIcon, PauseIcon, PlayIcon, PreviousIcon } from "./Icons";

export function MusicPanel() {
  const media = useMedia((s) => s.media);
  const send = useMedia((s) => s.send);
  const now = useNow(500, !!media?.playing);

  if (!media) {
    return <Empty icon={<MusicIcon />} title="没有正在播放的音乐" hint="在任意播放器里开始播放，就会显示在这里" />;
  }

  const elapsed = elapsedAt(media, now);
  const duration = media.duration;
  const progress = duration && elapsed != null ? elapsed / duration : null;

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
          <div className="music__artist">{media.artist || media.album || " "}</div>
        </div>

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
          <span className="music__app">{media.appName}</span>
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
