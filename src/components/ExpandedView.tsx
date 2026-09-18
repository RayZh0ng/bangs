import type { ComponentType, SVGProps } from "react";

import type { Size } from "../lib/layout";
import { useNotch, type Section } from "../store/notch";
import { DevPanel } from "./DevPanel";
import { ClipboardIcon, CodeIcon, MusicIcon, PinIcon, ShelfIcon, TimerIcon } from "./Icons";
import { MusicPanel } from "./MusicPanel";
import { PastePanel } from "./PastePanel";
import { ShelfPanel } from "./ShelfPanel";
import { TimerPanel } from "./TimerPanel";
import { usePaste } from "../store/paste";

const SECTIONS: { id: Section; label: string; Icon: ComponentType<SVGProps<SVGSVGElement>> }[] = [
  { id: "music", label: "音乐", Icon: MusicIcon },
  { id: "timer", label: "专注", Icon: TimerIcon },
  { id: "shelf", label: "暂存", Icon: ShelfIcon },
  { id: "dev", label: "代码", Icon: CodeIcon },
  { id: "paste", label: "剪贴板", Icon: ClipboardIcon },
];

interface Props {
  base: Size;
  /** Width of the hardware notch the header must leave empty. */
  notchGap: number;
}

export function ExpandedView({ base, notchGap }: Props) {
  const section = useNotch((s) => s.section);
  const pinned = useNotch((s) => s.pinned);
  const hasPaste = usePaste((state) => state.available);
  const { selectSection, togglePin } = useNotch.getState();
  const sections = SECTIONS.filter((entry) => entry.id !== "paste" || hasPaste);

  return (
    <div className="expanded">
      <header className="bar" style={{ height: base.height }}>
        <nav className="bar__side">
          {sections.map(({ id, label, Icon }) => (
            <button
              key={id}
              className={`tab${section === id ? " is-active" : ""}`}
              onClick={() => selectSection(id)}
              title={label}
            >
              <Icon width={13} height={13} />
              {section === id && <span>{label}</span>}
            </button>
          ))}
        </nav>
        <div style={{ width: notchGap, flex: "none" }} />
        <div className="bar__side bar__side--end">
          <button
            className={`icon-button${pinned ? " is-active" : ""}`}
            onClick={togglePin}
            title={pinned ? "取消固定" : "固定展开"}
          >
            <PinIcon width={13} height={13} filled={pinned} />
          </button>
        </div>
      </header>

      <main className="panel">
        {section === "music" && <MusicPanel />}
        {section === "timer" && <TimerPanel />}
        {section === "shelf" && <ShelfPanel />}
        {section === "dev" && <DevPanel />}
        {section === "paste" && <PastePanel />}
      </main>
    </div>
  );
}
