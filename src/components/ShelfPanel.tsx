import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { PointerEvent } from "react";

import { t } from "../lib/i18n";
import { native } from "../lib/native";
import { useNotch } from "../store/notch";
import { SHELF_CAPACITY, useShelf, type ShelfItem } from "../store/shelf";
import { Empty } from "./Empty";
import { CloseIcon, FolderIcon, ShelfIcon } from "./Icons";

const DRAG_THRESHOLD_PX = 5;

export function ShelfPanel() {
  const items = useShelf((s) => s.items);
  const clear = useShelf((s) => s.clear);

  if (!items.length) {
    return <Empty icon={<ShelfIcon />} title={t("暂存架是空的", "The shelf is empty")}
        hint={t("把文件拖到刘海上暂存，需要时再拖出来", "Drop files on the notch to park them, drag them back out later")} />;
  }

  return (
    <div className="shelf">
      <div className="shelf__row">
        {items.map((item) => (
          <ShelfTile key={item.path} item={item} />
        ))}
      </div>
      <div className="shelf__footer">
        <span>
          {items.length}/{SHELF_CAPACITY} · {t("拖出使用，双击打开", "drag out to use, double-click to open")}
        </span>
        <button className="link-button" onClick={clear}>
          {t("清空", "Clear")}
        </button>
      </div>
    </div>
  );
}

function ShelfTile({ item }: { item: ShelfItem }) {
  const remove = useShelf((s) => s.remove);

  const beginDrag = (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 || (event.target as HTMLElement).closest("button")) return;
    const origin = { x: event.clientX, y: event.clientY };

    const cleanup = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", cleanup);
    };
    const move = (moveEvent: globalThis.PointerEvent) => {
      if (Math.hypot(moveEvent.clientX - origin.x, moveEvent.clientY - origin.y) < DRAG_THRESHOLD_PX) return;
      cleanup();
      const notch = useNotch.getState();
      notch.setDraggingOut(true);
      startDrag({ item: [item.path], icon: notch.dragIcon ?? item.path }, () => {
        useNotch.getState().setDraggingOut(false);
      }).catch((error) => {
        console.warn("drag failed", error);
        useNotch.getState().setDraggingOut(false);
      });
    };

    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", cleanup);
  };

  return (
    <div
      className="tile"
      title={item.path}
      onPointerDown={beginDrag}
      onDoubleClick={() => native.openFile(item.path).catch(() => {})}
    >
      <div className="tile__thumb">
        {item.isImage ? (
          <img src={convertFileSrc(item.path)} alt="" draggable={false} />
        ) : item.isDir ? (
          <FolderIcon width={24} height={24} />
        ) : (
          <span className="tile__ext">{item.extension.slice(0, 4) || "FILE"}</span>
        )}
      </div>
      <div className="tile__name">{item.name}</div>
      <div className="tile__actions">
        <button className="tile__action" title={t("在文件夹中显示", "Show in folder")} onClick={() => native.revealFile(item.path).catch(() => {})}>
          <FolderIcon width={10} height={10} />
        </button>
        <button className="tile__action" title={t("移出暂存架", "Remove")} onClick={() => remove(item.path)}>
          <CloseIcon width={10} height={10} />
        </button>
      </div>
    </div>
  );
}
