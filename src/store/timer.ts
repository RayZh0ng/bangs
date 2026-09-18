import { create } from "zustand";
import { persist } from "zustand/middleware";

export type TimerKind = "focus" | "break";

export const PRESETS: Record<TimerKind, number[]> = {
  focus: [15, 25, 45, 60],
  break: [5, 10, 15, 20],
};

interface TimerStore {
  kind: TimerKind;
  minutes: Record<TimerKind, number>;
  autoBreak: boolean;
  /** Unix ms when the running timer ends; null when idle or paused. */
  endsAt: number | null;
  /** Remaining ms while paused. */
  pausedRemaining: number | null;
  /** Length of the current run in ms. */
  duration: number;

  start(kind?: TimerKind): void;
  pause(): void;
  resume(): void;
  stop(): void;
  selectKind(kind: TimerKind): void;
  setMinutes(kind: TimerKind, minutes: number): void;
  setAutoBreak(autoBreak: boolean): void;
  /** Ends the run that just hit zero; returns the kind that finished. */
  complete(): TimerKind;
}

export const useTimer = create<TimerStore>()(
  persist(
    (set, get) => ({
      kind: "focus",
      minutes: { focus: 25, break: 5 },
      autoBreak: true,
      endsAt: null,
      pausedRemaining: null,
      duration: 0,

      start(kind = get().kind) {
        const duration = get().minutes[kind] * 60_000;
        set({ kind, duration, endsAt: Date.now() + duration, pausedRemaining: null });
      },

      pause() {
        const { endsAt } = get();
        if (endsAt === null) return;
        set({ endsAt: null, pausedRemaining: Math.max(0, endsAt - Date.now()) });
      },

      resume() {
        const { pausedRemaining } = get();
        if (pausedRemaining === null) return;
        set({ endsAt: Date.now() + pausedRemaining, pausedRemaining: null });
      },

      stop() {
        set({ endsAt: null, pausedRemaining: null, duration: 0 });
      },

      selectKind(kind) {
        if (!isTimerActive(get())) set({ kind });
      },

      setMinutes(kind, minutes) {
        set({ minutes: { ...get().minutes, [kind]: minutes } });
      },

      setAutoBreak(autoBreak) {
        set({ autoBreak });
      },

      complete() {
        const finished = get().kind;
        if (finished === "focus" && get().autoBreak) {
          get().start("break");
        } else {
          set({
            kind: finished === "focus" ? "break" : "focus",
            endsAt: null,
            pausedRemaining: null,
            duration: 0,
          });
        }
        return finished;
      },
    }),
    {
      name: "bangs.timer",
      partialize: ({ kind, minutes, autoBreak, endsAt, pausedRemaining, duration }) => ({
        kind,
        minutes,
        autoBreak,
        endsAt,
        pausedRemaining,
        duration,
      }),
    },
  ),
);

type TimerSnapshot = Pick<TimerStore, "endsAt" | "pausedRemaining">;

export function isTimerActive(timer: TimerSnapshot) {
  return timer.endsAt !== null || timer.pausedRemaining !== null;
}

export function timerRemaining(timer: TimerSnapshot, now: number) {
  return timer.pausedRemaining ?? Math.max(0, (timer.endsAt ?? now) - now);
}
