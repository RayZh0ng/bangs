import type { ReactNode } from "react";

import { relativeTime } from "../lib/format";
import { native, type AgentSession, type EditorWorkspace } from "../lib/native";
import { useNow } from "../lib/useNow";
import { useDev } from "../store/dev";

const STATUS_LABEL: Record<AgentSession["status"], string> = {
  busy: "运行中",
  waiting: "等你回复",
  idle: "空闲",
};

const AGENT_LABEL: Record<AgentSession["agent"], string> = {
  claude: "Claude",
  codex: "Codex",
};

const EDITORS: EditorWorkspace["editor"][] = ["code", "cursor"];

export function DevPanel() {
  const sessions = useDev((state) => state.sessions);
  const workspaces = useDev((state) => state.workspaces);
  const now = useNow(20_000);

  return (
    <div className="dev">
      {/* Claude and Codex share one list; each row says which one it is. */}
      <section className="dev__column">
        {sessions.length ? (
          <>
            <h4 className="dev__heading">会话</h4>
            {sessions.map((session) => (
              <button
                key={session.id}
                className="row row--tight"
                title={`${session.name} · ${session.path}`}
                onClick={() => native.openProject(session.path).catch(() => {})}
              >
                <span className={`dot dot--${session.status}`} />
                <span className="tag">{AGENT_LABEL[session.agent]}</span>
                <span className="row__title">{session.project}</span>
                <span className="row__meta">{sinceLabel(session, now)}</span>
              </button>
            ))}
          </>
        ) : (
          <p className="dev__empty">没有在跑的会话</p>
        )}
      </section>

      {/* Editors keep their own groups, one per app. */}
      <section className="dev__column">
        {workspaces.length ? (
          EDITORS.map((editor) => (
            <Group
              key={editor}
              rows={workspaces.filter((workspace) => workspace.editor === editor)}
            />
          ))
        ) : (
          <p className="dev__empty">没有打开的项目</p>
        )}
      </section>
    </div>
  );
}

function Group({ rows }: { rows: EditorWorkspace[] }): ReactNode {
  if (!rows.length) return null;
  return (
    <>
      <h4 className="dev__heading">{rows[0].editorName}</h4>
      {rows.map((workspace) => (
        <button
          key={workspace.path}
          className="row row--tight"
          title={`${workspace.editorName} · ${workspace.path}`}
          onClick={() => native.openProject(workspace.path, workspace.editor).catch(() => {})}
        >
          <span className={`dot${workspace.active ? " dot--front" : " dot--blank"}`} />
          <span className="row__title">{workspace.project}</span>
          <span className="row__meta">{workspace.branch ?? ""}</span>
        </button>
      ))}
    </>
  );
}

/** Idle sessions show when they stopped; the others how long they have been there. */
function sinceLabel(session: AgentSession, now: number) {
  const elapsed = relativeTime(session.updatedAt, now);
  if (session.status === "idle") return elapsed;
  if (elapsed === "刚刚") return STATUS_LABEL[session.status];
  return `已${session.status === "busy" ? "跑" : "等"} ${elapsed.replace("前", "")}`;
}
