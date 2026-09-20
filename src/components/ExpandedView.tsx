import type { ComponentType, SVGProps } from "react";



import { t } from "../lib/i18n";
import type { Size } from "../lib/layout";
import { useNotch, type Section } from "../store/notch";
import { DevPanel } from "./DevPanel";
import { ActivityPanel } from "./ActivityPanel";
import { BoardIcon, ClipboardIcon, CodeIcon, MusicIcon, PinIcon, ShelfIcon, TodoIcon } from "./Icons";
import { MusicPanel } from "./MusicPanel";
import { ClipboardPanel } from "./ClipboardPanel";
import { ShelfPanel } from "./ShelfPanel";
import { TodoPanel } from "./TodoPanel";
import { useActivities } from "../store/activities";
import { useClipboard } from "../store/clipboard";

const SECTIONS: { id: Section; label: () => string; Icon: ComponentType<SVGProps<SVGSVGElement>> }[] = [
  { id: "music", label: () => t("音乐", "Music"), Icon: MusicIcon },
  { id: "shelf", label: () => t("暂存", "Shelf"), Icon: ShelfIcon },
  { id: "dev", label: () => t("代码", "Code"), Icon: CodeIcon },
  { id: "paste", label: () => t("剪贴板", "Clipboard"), Icon: ClipboardIcon },
  { id: "todo", label: () => t("待办", "To-do"), Icon: TodoIcon },
  { id: "board", label: () => t("上岛", "Board"), Icon: BoardIcon },
];

interface Props {
  base: Size;
  /** Width of the hardware notch the header must leave empty. */
  notchGap: number;
}

export function ExpandedView({ base, notchGap }: Props) {
  const section = useNotch((s) => s.section);
  const pinned = useNotch((s) => s.pinned);
  const hasClipboard = useClipboard((state) => state.available);
  const hasActivities = useActivities((state) => state.items.length > 0);
  const isMac = useNotch((s) => s.screen.platform === "macos");
  const { selectSection, togglePin } = useNotch.getState();
  // The clipboard tab stays visible on macOS without Paste, to offer it.
  // The board only appears once something has docked, so the bar stays short.
  const sections = SECTIONS.filter((entry) =>
    entry.id === "paste" ? hasClipboard || isMac : entry.id !== "board" || hasActivities || section === "board",
  );

  return (
    <div className="expanded">
      <header className="bar" style={{ height: base.height }}>
        <nav className="bar__side">
          {sections.map(({ id, label, Icon }) => (
            <button
              key={id}
              className={`tab${section === id ? " is-active" : ""}`}
              onClick={() => selectSection(id)}
              title={label()}
            >
              <Icon width={13} height={13} />
              {section === id && <span>{label()}</span>}
            </button>
          ))}
        </nav>
        <div style={{ width: notchGap, flex: "none" }} />
        <div className="bar__side bar__side--end">
          <button
            className={`icon-button${pinned ? " is-active" : ""}`}
            onClick={togglePin}
            title={pinned ? t("取消固定", "Unpin") : t("固定展开", "Keep open")}
          >
            <PinIcon width={13} height={13} filled={pinned} />
          </button>
        </div>
      </header>

      {/* Keyed so switching tabs fades the new panel in. */}
      <main className="panel" key={section}>
        {section === "music" && <MusicPanel />}
        {section === "shelf" && <ShelfPanel />}
        {section === "dev" && <DevPanel />}
        {section === "paste" && <ClipboardPanel />}
        {section === "todo" && <TodoPanel />}
        {section === "board" && <ActivityPanel />}
      </main>
    </div>
  );
}
