import { t } from "./i18n";

const pad = (value: number) => String(value).padStart(2, "0");

/** Track position, e.g. `3:07` or `1:02:45`. */
export function clock(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const rest = total % 60;
  return hours > 0 ? `${hours}:${pad(minutes)}:${pad(rest)}` : `${minutes}:${pad(rest)}`;
}

type Unit = "minute" | "hour" | "day";

const ZH: Record<Unit, string> = { minute: "分钟", hour: "小时", day: "天" };

/** How long ago, rounded down; null when it is under a minute. */
function span(timestamp: number, now: number): { value: number; unit: Unit } | null {
  const seconds = Math.max(0, (now - timestamp) / 1000);
  if (seconds < 60) return null;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return { value: minutes, unit: "minute" };
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return { value: hours, unit: "hour" };
  return { value: Math.floor(hours / 24), unit: "day" };
}

/** Coarse "x minutes ago" used in the dev and clipboard lists. */
export function relativeTime(timestamp: number, now: number): string {
  const ago = span(timestamp, now);
  if (!ago) return t("刚刚", "just now");
  return t(`${ago.value} ${ZH[ago.unit]}前`, `${ago.value} ${ago.unit}${ago.value > 1 ? "s" : ""} ago`);
}

/** The same length of time without the "ago"; null when it is under a minute. */
export function duration(timestamp: number, now: number): string | null {
  const length = span(timestamp, now);
  if (!length) return null;
  return t(
    `${length.value} ${ZH[length.unit]}`,
    `${length.value} ${length.unit}${length.value > 1 ? "s" : ""}`,
  );
}
