import { create } from "zustand";

import { native, type ClipItem, type PasteState } from "../lib/native";

interface PasteStore extends PasteState {
  /** Id of the entry that was just copied, for the inline confirmation. */
  copiedId: number | null;
  update(next: PasteState): void;
  use(item: ClipItem): Promise<void>;
}

let copiedTimer: number | undefined;

export const usePaste = create<PasteStore>((set) => ({
  available: false,
  items: [],
  copiedId: null,

  update(next) {
    set({ available: next.available, items: next.items });
  },

  async use(item) {
    // Paste owns the richer pasteboard types, so hand those back to it.
    if (item.kind !== "text") {
      await native.pasteOpen().catch((error) => console.warn("open Paste failed", error));
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
}));
