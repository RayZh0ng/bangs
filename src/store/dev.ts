import { create } from "zustand";

import type { AgentSession, DevState } from "../lib/native";

interface DevStore extends DevState {
  /** Applies an update and reports the sessions that just stopped working. */
  update(next: DevState): AgentSession[];
}

export const useDev = create<DevStore>((set, get) => ({
  sessions: [],
  workspaces: [],

  update(next) {
    const before = new Map(get().sessions.map((session) => [session.id, session.status]));
    const finished = next.sessions.filter(
      (session) => before.get(session.id) === "busy" && session.status !== "busy",
    );
    set({ sessions: next.sessions, workspaces: next.workspaces });
    return finished;
  },
}));

export const waitingSessions = (sessions: AgentSession[]) =>
  sessions.filter((session) => session.status === "waiting");
