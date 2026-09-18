import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface ScreenInfo {
  platform: string;
  hasNotch: boolean;
  notchWidth: number;
  notchHeight: number;
  menuBarHeight: number;
  displayName: string;
}

export interface Settings {
  visible: boolean;
  expandOnHover: boolean;
  idleHandle: boolean;
  notifyClaudeIdle: boolean;
  display: string | null;
}

export interface MediaState {
  title: string;
  artist: string;
  album: string;
  appName: string;
  sourceId: string;
  playing: boolean;
  /** Seconds. */
  duration: number | null;
  /** Seconds, sampled at `elapsedAt`. */
  elapsed: number | null;
  /** Unix milliseconds. */
  elapsedAt: number;
  /** Data URL. */
  artwork: string | null;
}

export interface FileMeta {
  path: string;
  name: string;
  extension: string;
  size: number;
  isDir: boolean;
  isImage: boolean;
}

export type SessionStatus = "busy" | "waiting" | "idle";

export type Agent = "claude" | "codex";

export interface AgentSession {
  id: string;
  agent: Agent;
  name: string;
  path: string;
  project: string;
  status: SessionStatus;
  detail: string | null;
  /** Unix milliseconds of the last status change. */
  updatedAt: number;
}

export interface EditorWorkspace {
  editor: "code" | "cursor";
  editorName: string;
  path: string;
  project: string;
  branch: string | null;
  active: boolean;
}

export interface DevState {
  sessions: AgentSession[];
  workspaces: EditorWorkspace[];
}

export type ClipKind = "text" | "image" | "files";

export interface ClipItem {
  id: number;
  kind: ClipKind;
  preview: string;
  app: string | null;
  /** Source app icon as a data URL. */
  icon: string | null;
  pinned: boolean;
  /** Unix milliseconds. */
  createdAt: number;
}

export interface PasteState {
  available: boolean;
  items: ClipItem[];
}

export interface Bootstrap {
  screen: ScreenInfo;
  settings: Settings;
  media: MediaState | null;
  dev: DevState;
  paste: PasteState;
  dragIcon: string | null;
}

export type MediaCommand =
  | { action: "toggle" }
  | { action: "next" }
  | { action: "previous" }
  | { action: "seek"; position: number };

export const native = {
  bootstrap: () => invoke<Bootstrap>("bootstrap"),
  ready: () => invoke<void>("notch_ready"),
  setHitRect: (width: number, height: number) => invoke<void>("set_hit_rect", { width, height }),
  media: (command: MediaCommand) => invoke<void>("media_command", { command }),
  openProject: (path: string, editor?: string) => invoke<void>("open_project", { path, editor }),
  pasteCopy: (id: number) => invoke<void>("paste_copy", { id }),
  pasteShow: () => invoke<void>("paste_show"),
  inspectFiles: (paths: string[]) => invoke<FileMeta[]>("shelf_inspect", { paths }),
  openFile: (path: string) => invoke<void>("open_file", { path }),
  revealFile: (path: string) => invoke<void>("reveal_file", { path }),
};

export const events = {
  hover: (handler: (inside: boolean) => void) =>
    listen<boolean>("bangs://hover", (event) => handler(event.payload)),
  /** Window-local pointer position while the cursor is over the notch. */
  pointer: (handler: (point: [number, number]) => void) =>
    listen<[number, number]>("bangs://pointer", (event) => handler(event.payload)),
  outsideClick: (handler: () => void) => listen("bangs://outside-click", () => handler()),
  screen: (handler: (screen: ScreenInfo) => void) =>
    listen<ScreenInfo>("bangs://screen", (event) => handler(event.payload)),
  settings: (handler: (settings: Settings) => void) =>
    listen<Settings>("settings://changed", (event) => handler(event.payload)),
  media: (handler: (media: MediaState | null) => void) =>
    listen<MediaState | null>("media://update", (event) => handler(event.payload)),
  dev: (handler: (dev: DevState) => void) =>
    listen<DevState>("bangs://dev", (event) => handler(event.payload)),
  paste: (handler: (paste: PasteState) => void) =>
    listen<PasteState>("bangs://paste", (event) => handler(event.payload)),
};
