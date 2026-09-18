import { create } from "zustand";

import { native, type ClipItem, type PasteState } from "../lib/native";

interface PasteStore extends PasteState {
  /** Id of the entry that was just copied, for the inline confirmation. */
  copiedId: number | null;
  /** Transient message shown in the footer. */
  notice: string | null;
  update(next: PasteState): void;
  use(item: ClipItem): Promise<void>;
  showPanel(): Promise<void>;
}

let copiedTimer: number | undefined;
let noticeTimer: number | undefined;

export const usePaste = create<PasteStore>((set) => ({
  available: false,
  items: [],
  copiedId: null,
  notice: null,

  update(next) {
    set({ available: next.available, items: next.items });
  },

  async use(item) {
    // Paste owns the richer pasteboard types, so let it handle those.
    if (item.kind !== "text") {
      await usePaste.getState().showPanel();
      return;
    }
    try {
      await native.pasteCopy(item.id);
      window.clearTimeout(copiedTimer);
      set({ copiedId: item.id });
      copiedTimer = window.setTimeout(() => set({ copiedId: null }), 1400);
    } catch (error) {
      console.warn("copy failed", error);
    }
  },

  async showPanel() {
    try {
      await native.pasteShow();
    } catch (error) {
      window.clearTimeout(noticeTimer);
      set({ notice: String(error) });
      noticeTimer = window.setTimeout(() => set({ notice: null }), 4000);
    }
  },
}));
