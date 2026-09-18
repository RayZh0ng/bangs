import type { ReactNode } from "react";

import { relativeTime } from "../lib/format";
import { native, type ClaudeSession } from "../lib/native";
import { useNow } from "../lib/useNow";
import { useDev } from "../store/dev";

const STATUS_LABEL: Record<ClaudeSession["status"], string> = {
  busy: "运行中",
  waiting: "等你回复",
  idle: "空闲",
};

export function DevPanel() {
  const sessions = useDev((state) => state.sessions);
  const workspaces = useDev((state) => state.workspaces);
  const now = useNow(20_000);

  return (
    <div className="dev">
      <Column title="Claude" empty={!sessions.length && "没有在跑的会话"}>
        {sessions.map((session) => (
          <button
            key={session.id}
            className="row row--tight"
            title={`${session.name} · ${session.path}`}
            onClick={() => native.openProject(session.path).catch(() => {})}
          >
            <span className={`dot dot--${session.status}`} />
            <span className="row__title">{session.project}</span>
            <span className="row__meta">{sinceLabel(session, now)}</span>
          </button>
        ))}
      </Column>

      <Column title="编辑器" empty={!workspaces.length && "没有打开的项目"}>
        {workspaces.map((workspace) => (
          <button
            key={`${workspace.editor}:${workspace.path}`}
            className="row row--tight"
            title={`${workspace.editorName} · ${workspace.path}`}
            onClick={() => native.openProject(workspace.path, workspace.editor).catch(() => {})}
          >
            <span className={`mark mark--${workspace.editor}${workspace.active ? " is-active" : ""}`} />
            <span className="row__title">{workspace.project}</span>
            <span className="row__meta">{workspace.branch ?? workspace.editorName}</span>
          </button>
        ))}
      </Column>
    </div>
  );
}

function Column({ title, empty, children }: { title: string; empty: string | false; children: ReactNode }) {
  return (
    <section className="dev__column">
      <h4 className="dev__heading">{title}</h4>
      {empty ? <p className="dev__empty">{empty}</p> : <div className="rows">{children}</div>}
    </section>
  );
}

/** Idle sessions show when they stopped; the others how long they have been there. */
function sinceLabel(session: ClaudeSession, now: number) {
  const elapsed = relativeTime(session.updatedAt, now);
  if (session.status === "idle") return elapsed;
  if (elapsed === "刚刚") return STATUS_LABEL[session.status];
  return `已${session.status === "busy" ? "跑" : "等"} ${elapsed.replace("前", "")}`;
}
