import { primeAudio } from "../lib/alerts";
import { countdown } from "../lib/format";
import { useNow } from "../lib/useNow";
import { PRESETS, isTimerActive, timerRemaining, useTimer, type TimerKind } from "../store/timer";

const KIND_LABEL: Record<TimerKind, string> = { focus: "专注", break: "休息" };
const RING_RADIUS = 50;
const RING_LENGTH = 2 * Math.PI * RING_RADIUS;

export function TimerPanel() {
  const timer = useTimer();
  const running = timer.endsAt !== null;
  const active = isTimerActive(timer);
  const now = useNow(250, running);

  const remaining = active ? timerRemaining(timer, now) : timer.minutes[timer.kind] * 60_000;
  const progress = active && timer.duration > 0 ? 1 - remaining / timer.duration : 0;

  return (
    <div className={`timer timer--${timer.kind}`}>
      <div className="ring">
        <svg viewBox="0 0 120 120" width={120} height={120}>
          <circle className="ring__track" cx="60" cy="60" r={RING_RADIUS} />
          <circle
            className="ring__value"
            cx="60"
            cy="60"
            r={RING_RADIUS}
            strokeDasharray={RING_LENGTH}
            strokeDashoffset={RING_LENGTH * (1 - progress)}
          />
        </svg>
        <div className="ring__center">
          <span className="ring__time">{countdown(remaining)}</span>
          <span className="ring__label">
            {active ? (running ? `${KIND_LABEL[timer.kind]}中` : "已暂停") : KIND_LABEL[timer.kind]}
          </span>
        </div>
      </div>

      <div className="timer__side">
        <div className="timer__row">
          <div className="segmented">
            {(["focus", "break"] as const).map((kind) => (
              <button
                key={kind}
                className={timer.kind === kind ? "is-active" : ""}
                disabled={active}
                onClick={() => timer.selectKind(kind)}
              >
                {KIND_LABEL[kind]}
              </button>
            ))}
          </div>
          <label className="check">
            <input
              type="checkbox"
              checked={timer.autoBreak}
              onChange={(event) => timer.setAutoBreak(event.target.checked)}
            />
            专注后自动休息
          </label>
        </div>

        <div className="chips">
          {PRESETS[timer.kind].map((minutes) => (
            <button
              key={minutes}
              className={`chip${timer.minutes[timer.kind] === minutes ? " is-active" : ""}`}
              disabled={active}
              onClick={() => timer.setMinutes(timer.kind, minutes)}
            >
              {minutes} 分
            </button>
          ))}
        </div>

        <div className="actions">
          {!active && (
            <button
              className="button button--primary"
              onClick={() => {
                primeAudio();
                timer.start();
              }}
            >
              开始{KIND_LABEL[timer.kind]}
            </button>
          )}
          {active && (
            <button className="button button--primary" onClick={running ? timer.pause : timer.resume}>
              {running ? "暂停" : "继续"}
            </button>
          )}
          {active && (
            <button className="button" onClick={timer.stop}>
              结束
            </button>
          )}
          {active && timer.kind === "focus" && (
            <button className="button" onClick={() => timer.start("break")}>
              直接休息
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
