import type { DragDropEvent } from "@tauri-apps/api/webview";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useEffect } from "react";

import { Notch } from "./components/Notch";
import { notify, playPing } from "./lib/alerts";
import { hoverAt } from "./lib/hover";
import { events, native, type DevState } from "./lib/native";
import { useDev } from "./store/dev";
import { useMedia } from "./store/media";
import { usePaste } from "./store/paste";
import { useNotch, type Section } from "./store/notch";
import { useShelf, type AddResult } from "./store/shelf";

export default function App() {
  const ready = useNotch((s) => s.ready);

  useEffect(() => {
    let disposed = false;
    const subscriptions = [
      events.hover((inside) => {
        if (!inside) hoverAt(null);
        useNotch.getState().hoverChanged(inside);
      }),
      events.pointer(hoverAt),
      events.outsideClick(() => useNotch.getState().outsideClicked()),
      events.screen((screen) => useNotch.setState({ screen })),
      events.settings((settings) => useNotch.setState({ settings })),
      events.media((media) => useMedia.getState().update(media)),
      events.dev(handleDev),
      events.paste((paste) => usePaste.getState().update(paste)),
      getCurrentWebview().onDragDropEvent((event) => void handleDragDrop(event.payload)),
    ];

    // Subscribe first, then snapshot, so no update falls in between.
    Promise.all(subscriptions)
      .then(async () => {
        const boot = await native.bootstrap();
        if (disposed) return;
        useMedia.getState().update(boot.media);
        useDev.getState().update(boot.dev);
        usePaste.getState().update(boot.paste);
        await useShelf.getState().refresh();
        useNotch.getState().init(boot);
        // Not requestAnimationFrame: hidden webviews never run frame callbacks.
        window.setTimeout(() => void native.ready(), 50);
      })
      .catch((error) => console.error("bootstrap failed", error));

    return () => {
      disposed = true;
      subscriptions.forEach((subscription) => subscription.then((unlisten) => unlisten()));
    };
  }, []);

  return ready ? <Notch /> : null;
}

async function handleDragDrop(event: DragDropEvent) {
  const notch = useNotch.getState();
  switch (event.type) {
    case "enter":
      if (event.paths.length) notch.dragEntered();
      break;
    case "leave":
      notch.dragLeft();
      break;
    case "drop":
      if (notch.draggingOut) break;
      notch.dropped(dropMessage(await useShelf.getState().add(event.paths)));
      break;
  }
}

/** How long an alert keeps the panel open when nobody looks at it. */
const ALERT_OPEN_MS = 8_000;
let alertTimer: number | undefined;

/** Opens the notch for an alert, then gets out of the way on its own. */
function alertWith(section: Section) {
  useNotch.getState().expand(section, true);
  window.clearTimeout(alertTimer);
  alertTimer = window.setTimeout(() => {
    const notch = useNotch.getState();
    if (notch.mode === "expanded" && !notch.hovering) notch.collapse();
  }, ALERT_OPEN_MS);
}

/** Claude Code sessions run unattended, so surface the ones that stopped. */
function handleDev(next: DevState) {
  const finished = useDev.getState().update(next);
  const notch = useNotch.getState();
  if (!notch.settings.notifyClaudeIdle) return;

  const session = finished.find((entry) => entry.status === "waiting") ?? finished[0];
  if (!session) return;
  playPing();
  void notify(
    session.status === "waiting" ? `${session.project} 在等你回复` : `${session.project} 跑完了`,
    session.detail ?? session.name,
  );
  alertWith("dev");
}

function dropMessage({ added, duplicates, rejectedForSpace }: AddResult) {
  if (added > 0) return rejectedForSpace > 0 ? `已暂存 ${added} 个，暂存架满了` : `已暂存 ${added} 个文件`;
  if (rejectedForSpace > 0) return "暂存架满了";
  return duplicates > 0 ? "已经在暂存架里" : "无法暂存这个项目";
}
