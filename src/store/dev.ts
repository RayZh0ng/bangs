import { create } from "zustand";

import type { ClaudeSession, DevState } from "../lib/native";

interface DevStore extends DevState {
  /** Applies an update and reports the sessions that just stopped working. */
  update(next: DevState): ClaudeSession[];
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

export const waitingSessions = (sessions: ClaudeSession[]) =>
  sessions.filter((session) => session.status === "waiting");

export const busySessions = (sessions: ClaudeSession[]) =>
  sessions.filter((session) => session.status === "busy");
