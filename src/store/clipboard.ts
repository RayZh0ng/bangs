import { create } from "zustand";

import { native, type ClipItem, type ClipboardState } from "../lib/native";

interface ClipboardStore extends ClipboardState {
  /** Id of the entry that was just copied, for the inline confirmation. */
  copiedId: number | null;
  /** Transient message shown in the footer. */
  notice: string | null;
  /** False once the history is fully loaded. */
  hasMore: boolean;
  loading: boolean;
  update(next: ClipboardState): void;
  loadMore(): Promise<void>;
  use(item: ClipItem): Promise<void>;
  showPanel(): Promise<void>;
  clear(): Promise<void>;
  install(): Promise<void>;
}

let copiedTimer: number | undefined;
let noticeTimer: number | undefined;

function report(set: (partial: Partial<ClipboardStore>) => void, error: unknown) {
  window.clearTimeout(noticeTimer);
  set({ notice: String(error) });
  noticeTimer = window.setTimeout(() => set({ notice: null }), 4000);
}

export const useClipboard = create<ClipboardStore>((set, get) => ({
  available: false,
  source: "builtin",
  items: [],
  copiedId: null,
  notice: null,
  hasMore: true,
  loading: false,

  update(next) {
    set({ available: next.available, source: next.source, items: next.items });
  },

  async loadMore() {
    if (get().loading || !get().hasMore) return;
    set({ loading: true });
    try {
      const more = await native.clipboardMore();
      set({ hasMore: more });
    } catch (error) {
      report(set, error);
    } finally {
      set({ loading: false });
    }
  },

  async use(item) {
    // A picture is copied like anything else; files are promises only Paste
    // can keep, so those are handed over to its own panel.
    if (item.kind === "files" && get().source === "paste") {
      await get().showPanel();
      return;
    }
    try {
      await native.clipboardUse(item.id);
      window.clearTimeout(copiedTimer);
      set({ copiedId: item.id });
      copiedTimer = window.setTimeout(() => set({ copiedId: null }), 1400);
    } catch (error) {
      report(set, error);
    }
  },

  async showPanel() {
    await native.clipboardOpen().catch((error) => report(set, error));
  },

  async clear() {
    await native.clipboardClear().catch((error) => report(set, error));
  },

  async install() {
    await native.clipboardInstall().catch((error) => report(set, error));
  },
}));
