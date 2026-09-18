import { create } from "zustand";

import type { AgentSession, DevState } from "../lib/native";

/**
 * A session that keeps stopping and starting — one being driven by hand, say —
 * must not pop the notch open every few seconds, so each one is only worth
 * announcing this often.
 */
const ANNOUNCE_EVERY_MS = 3 * 60_000;
/** Work that was over almost as soon as it started is not worth announcing. */
const WORTH_ANNOUNCING_MS = 20_000;

interface DevStore extends DevState {
  /** Applies an update and reports the sessions worth announcing. */
  update(next: DevState): AgentSession[];
}

/** Session id to when it was last announced, and when it started working. */
const announced = new Map<string, number>();
const busySince = new Map<string, number>();

export const useDev = create<DevStore>((set, get) => ({
  sessions: [],
  workspaces: [],

  update(next) {
    const now = Date.now();
    const before = new Map(get().sessions.map((session) => [session.id, session.status]));
    const finished: AgentSession[] = [];

    for (const session of next.sessions) {
      const was = before.get(session.id);
      if (session.status === "busy") {
        if (was !== "busy") busySince.set(session.id, now);
        continue;
      }
      if (was !== "busy") continue;

      const worked = now - (busySince.get(session.id) ?? now);
      busySince.delete(session.id);
      const last = announced.get(session.id) ?? 0;
      if (worked < WORTH_ANNOUNCING_MS || now - last < ANNOUNCE_EVERY_MS) continue;
      announced.set(session.id, now);
      finished.push(session);
    }

    // Sessions that went away take their history with them.
    const live = new Set(next.sessions.map((session) => session.id));
    for (const id of [...announced.keys()]) if (!live.has(id)) announced.delete(id);
    for (const id of [...busySince.keys()]) if (!live.has(id)) busySince.delete(id);

    set({ sessions: next.sessions, workspaces: next.workspaces });
    return finished;
  },
}));

export const waitingSessions = (sessions: AgentSession[]) =>
  sessions.filter((session) => session.status === "waiting");
