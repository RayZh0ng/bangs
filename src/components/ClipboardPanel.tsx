import type { UIEvent } from "react";

import { relativeTime } from "../lib/format";
import type { ClipItem } from "../lib/native";
import { useNow } from "../lib/useNow";
import { useClipboard } from "../store/clipboard";
import { Empty } from "./Empty";
import { ClipboardIcon } from "./Icons";

const KIND_LABEL: Record<ClipItem["kind"], string | null> = {
  text: null,
  image: "图片",
  files: "文件",
};

export function ClipboardPanel() {
  const items = useClipboard((state) => state.items);
  const available = useClipboard((state) => state.available);
  const source = useClipboard((state) => state.source);
  const copiedId = useClipboard((state) => state.copiedId);
  const notice = useClipboard((state) => state.notice);
  const hasMore = useClipboard((state) => state.hasMore);
  const { use, showPanel, clear, install, loadMore } = useClipboard.getState();
  const now = useNow(20_000);

  // Only macOS has an external history, and only there can it be missing.
  if (!available) {
    return (
      <div className="empty">
        <ClipboardIcon />
        <div className="empty__title">还没有剪贴板历史</div>
        <div className="empty__hint">装上 Paste，复制过的内容就会出现在这里</div>
        <button className="button" onClick={() => void install()}>
          去安装 Paste
        </button>
      </div>
    );
  }
  if (!items.length) {
    return <Empty icon={<ClipboardIcon />} title="剪贴板历史是空的" hint="复制点什么，就会出现在这里" />;
  }

  // Older entries load as the list reaches its end.
  const onScroll = (event: UIEvent<HTMLDivElement>) => {
    const list = event.currentTarget;
    if (list.scrollTop + list.clientHeight >= list.scrollHeight - 40) void loadMore();
  };

  return (
    <div className="paste">
      <div className="rows rows--snap" onScroll={onScroll}>
        {items.map((item) => (
          <button key={item.id} className="row" title={item.preview} onClick={() => void use(item)}>
            {item.icon ? (
              <img className="row__icon" src={item.icon} alt="" />
            ) : (
              <span className="row__icon row__icon--blank" />
            )}
            <span className="row__main">
              <span className="row__title row__title--plain">
                {KIND_LABEL[item.kind] && <span className="tag tag--soft">{KIND_LABEL[item.kind]}</span>}
                {item.preview === KIND_LABEL[item.kind] ? "" : item.preview}
              </span>
            </span>
            <span className={`row__meta${copiedId === item.id ? " row__meta--ok" : ""}`}>
              {copiedId === item.id ? "已复制" : relativeTime(item.createdAt, now)}
            </span>
          </button>
        ))}
      </div>
      <div className="shelf__footer">
        <span>{notice ?? `${items.length} 条${hasMore ? "+" : ""} · 点一下放回剪贴板`}</span>
        {source === "paste" ? (
          <button className="link-button" onClick={() => void showPanel()}>
            唤出 Paste
          </button>
        ) : (
          <button className="link-button" onClick={() => void clear()}>
            清空
          </button>
        )}
      </div>
    </div>
  );
}
