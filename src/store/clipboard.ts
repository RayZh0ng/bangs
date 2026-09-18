import { create } from "zustand";

import { native, type ClipItem, type ClipboardState } from "../lib/native";

interface ClipboardStore extends ClipboardState {
  /** Id of the entry that was just copied, for the inline confirmation. */
  copiedId: number | null;
  /** Transient message shown in the footer. */
  notice: string | null;
  update(next: ClipboardState): void;
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

  update(next) {
    set({ available: next.available, source: next.source, items: next.items });
  },

  async use(item) {
    // Paste owns the richer pasteboard types, so let it handle those.
    if (item.kind !== "text" && get().source === "paste") {
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
