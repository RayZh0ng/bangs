import { useEffect, type CSSProperties } from "react";

import { baseNotch, cornerRadii, hitRect, notchSize } from "../lib/layout";
import { native } from "../lib/native";
import { useNow } from "../lib/useNow";
import { waitingSessions, useDev } from "../store/dev";
import { isMediaLive, useMedia } from "../store/media";
import { useNotch } from "../store/notch";
import { useShelf } from "../store/shelf";
import { isTimerActive, timerRemaining, useTimer } from "../store/timer";
import { CompactView, type Activity } from "./CompactView";
import { DropView } from "./DropView";
import { ExpandedView } from "./ExpandedView";

export function Notch() {
  const mode = useNotch((s) => s.mode);
  const screen = useNotch((s) => s.screen);
  const settings = useNotch((s) => s.settings);
  const hovering = useNotch((s) => s.hovering);
  const media = useMedia((s) => s.media);
  const lastActiveAt = useMedia((s) => s.lastActiveAt);
  const timer = useTimer();
  const shelfCount = useShelf((s) => s.items.length);
  const waiting = useDev((s) => waitingSessions(s.sessions).length);

  const timerActive = isTimerActive(timer);
  const now = useNow(timerActive && timer.endsAt !== null ? 250 : 5_000);

  const activity: Activity = {
    attention: waiting,
    media: isMediaLive(media, lastActiveAt, now) ? media : null,
    timer: timerActive
      ? { kind: timer.kind, remaining: timerRemaining(timer, now), paused: timer.pausedRemaining !== null }
      : null,
    shelfCount,
  };
  const hasActivity =
    activity.attention > 0 || !!activity.media || !!activity.timer || activity.shelfCount > 0;

  const base = baseNotch(screen);
  const size = notchSize(mode, screen, settings, hasActivity);
  const hit = hitRect(size);
  const { ear, bottom } = cornerRadii(mode, size);

  useEffect(() => {
    native.setHitRect(hit.width, hit.height).catch(() => {});
  }, [hit.width, hit.height]);

  const style = {
    width: size.width,
    height: size.height,
    "--ear": `${ear}px`,
    "--radius": `${bottom}px`,
  } as CSSProperties;

  return (
    <div className="stage">
      <div
        className={`notch notch--${mode}${hovering ? " is-hovering" : ""}`}
        style={style}
        onClick={mode === "compact" ? () => useNotch.getState().expand(undefined, true) : undefined}
      >
        <div className="notch__clip">
          {/* Content is laid out at the target size so it never reflows while the shape animates. */}
          <div key={mode === "success" ? "drop" : mode} className="notch__content" style={{ width: size.width, height: size.height }}>
            {mode === "compact" && <CompactView activity={activity} base={base} />}
            {mode === "expanded" && <ExpandedView base={base} notchGap={screen.hasNotch ? base.width : 0} />}
            {(mode === "drop" || mode === "success") && <DropView base={base} success={mode === "success"} />}
          </div>
        </div>
      </div>
    </div>
  );
}
