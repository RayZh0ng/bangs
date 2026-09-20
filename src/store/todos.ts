import { create } from "zustand";

import { native, type Todo } from "../lib/native";

/** How long a line takes to blow away; in step with `dust` in styles.css. */
const DUST_MS = 940;

interface TodoStore {
  items: Todo[];
  /** Lines coming apart right now, so the panel can draw the dust. */
  dusting: string[];
  /** True while a field in the panel holds the keyboard; see TodoPanel. */
  typing: boolean;
  update(items: Todo[]): void;
  add(text: string): Promise<void>;
  snap(id: string): void;
  setTyping(typing: boolean): void;
}

export const useTodos = create<TodoStore>((set, get) => ({
  items: [],
  dusting: [],
  typing: false,

  update(items) {
    // Whatever the native list no longer has is done blowing away.
    const alive = new Set(items.map((item) => item.id));
    set({ items, dusting: get().dusting.filter((id) => alive.has(id)) });
  },

  async add(text) {
    if (!text.trim()) return;
    await native.todoAdd(text).catch((error) => console.warn("todo add failed", error));
  },

  snap(id) {
    if (get().dusting.includes(id)) return;
    set({ dusting: [...get().dusting, id] });
    // The line goes for good, but only once there is nothing left of it:
    // the native list is what makes the row disappear.
    window.setTimeout(() => {
      native.todoRemove(id).catch((error) => {
        console.warn("todo remove failed", error);
        // Nothing took the row away, so put it back rather than leave an
        // invisible line on the list.
        set({ dusting: get().dusting.filter((dusted) => dusted !== id) });
      });
    }, DUST_MS);
  },

  setTyping(typing) {
    // Asking twice would have the native side remember the notch itself as
    // the window to hand focus back to.
    if (get().typing === typing) return;
    set({ typing });
    native.captureKeyboard(typing).catch((error) => console.warn("keyboard capture failed", error));
  },
}));
