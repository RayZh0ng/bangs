import { useEffect, useState } from "react";

/** Wall time by default for session/clipboard ages; playback opts into monotonic time. */
export function useNow(intervalMs: number, active = true, monotonic = false): number {
  const readNow = () => monotonic ? performance.now() : Date.now();
  const [now, setNow] = useState(readNow);

  useEffect(() => {
    if (!active) return;
    setNow(readNow());
    const id = window.setInterval(() => setNow(readNow()), intervalMs);
    return () => window.clearInterval(id);
  }, [intervalMs, active, monotonic]);

  return now;
}
