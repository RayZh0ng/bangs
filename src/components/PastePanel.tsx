import { relativeTime } from "../lib/format";
import type { ClipItem } from "../lib/native";
import { useNow } from "../lib/useNow";
import { usePaste } from "../store/paste";
import { Empty } from "./Empty";
import { ClipboardIcon } from "./Icons";

const KIND_LABEL: Record<ClipItem["kind"], string | null> = {
  text: null,
  image: "图片",
  files: "文件",
};

export function PastePanel() {
  const items = usePaste((state) => state.items);
  const available = usePaste((state) => state.available);
  const copiedId = usePaste((state) => state.copiedId);
  const use = usePaste((state) => state.use);
  const notice = usePaste((state) => state.notice);
  const showPanel = usePaste((state) => state.showPanel);
  const now = useNow(20_000);

  if (!available) {
    return <Empty icon={<ClipboardIcon />} title="没找到 Paste" hint="装上 Paste 并复制过内容后，历史会显示在这里" />;
  }
  if (!items.length) {
    return <Empty icon={<ClipboardIcon />} title="剪贴板历史是空的" hint="复制点什么，就会出现在这里" />;
  }

  return (
    <div className="paste">
      <div className="rows rows--snap">
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
        <span>{notice ?? "点一下放回剪贴板"}</span>
        <button className="link-button" onClick={() => void showPanel()}>
          唤出 Paste
        </button>
      </div>
    </div>
  );
}
