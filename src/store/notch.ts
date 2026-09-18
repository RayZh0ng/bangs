import { create } from "zustand";

import type { Mode } from "../lib/layout";
import type { Bootstrap, ScreenInfo, Settings } from "../lib/native";
import { waitingSessions, useDev } from "./dev";
import { isMediaLive, useMedia } from "./media";

export type Section = "music" | "shelf" | "dev" | "paste";

const COLLAPSE_DELAY_MS = 450;
const DROP_LEAVE_DELAY_MS = 150;
const SUCCESS_MS = 1300;

interface NotchStore {
  ready: boolean;
  screen: ScreenInfo;
  settings: Settings;
  dragIcon: string | null;
  mode: Mode;
  section: Section;
  /** Pinned panels stay open until an outside click or the pin is released. */
  pinned: boolean;
  hovering: boolean;
  /** A shelf file is being dragged out; keep the panel open until it lands. */
  draggingOut: boolean;
  successText: string;

  init(boot: Bootstrap): void;
  hoverChanged(inside: boolean): void;
  outsideClicked(): void;
  expand(section?: Section, pin?: boolean): void;
  collapse(): void;
  togglePin(): void;
  selectSection(section: Section): void;
  dragEntered(): void;
  dragLeft(): void;
  dropped(message: string): void;
  setDraggingOut(dragging: boolean): void;
}

let collapseTimer: number | undefined;
let dropLeaveTimer: number | undefined;
let successTimer: number | undefined;
let modeBeforeDrop: Mode = "compact";

function clearTimers() {
  window.clearTimeout(collapseTimer);
  window.clearTimeout(dropLeaveTimer);
  window.clearTimeout(successTimer);
}

export const useNotch = create<NotchStore>((set, get) => {
  const scheduleCollapse = () => {
    window.clearTimeout(collapseTimer);
    collapseTimer = window.setTimeout(() => {
      const { mode, hovering, pinned, draggingOut } = get();
      if (mode === "expanded" && !hovering && !pinned && !draggingOut) get().collapse();
    }, COLLAPSE_DELAY_MS);
  };

  /** Opens on whatever is happening now, falling back to the last section. */
  const relevantSection = (): Section => {
    // A session waiting on an answer is the most urgent thing on screen.
    if (waitingSessions(useDev.getState().sessions).length) return "dev";
    const { media, lastActiveAt } = useMedia.getState();
    if (media?.playing) return "music";
    if (isMediaLive(media, lastActiveAt, Date.now())) return "music";
    return get().section;
  };

  return {
    ready: false,
    screen: { platform: "", hasNotch: false, notchWidth: 0, notchHeight: 0, menuBarHeight: 0, displayName: "" },
    settings: { visible: true, expandOnHover: true, idleHandle: false, notifyClaudeIdle: true, display: null },
    dragIcon: null,
    mode: "compact",
    section: "music",
    pinned: false,
    hovering: false,
    draggingOut: false,
    successText: "",

    init(boot) {
      set({ ready: true, screen: boot.screen, settings: boot.settings, dragIcon: boot.dragIcon });
    },

    hoverChanged(inside) {
      set({ hovering: inside });
      const { mode, settings, pinned } = get();
      if (mode === "drop" || mode === "success") return;
      if (inside) {
        window.clearTimeout(collapseTimer);
        if (mode === "compact" && settings.expandOnHover) get().expand(relevantSection());
      } else if (mode === "expanded" && !pinned) {
        scheduleCollapse();
      }
    },

    outsideClicked() {
      // A drag-out that never reported back must not keep the panel open.
      set({ draggingOut: false });
      if (get().mode === "expanded") get().collapse();
    },

    expand(section, pin = false) {
      window.clearTimeout(collapseTimer);
      const current = get();
      set({
        mode: "expanded",
        section: section ?? (current.mode === "compact" ? relevantSection() : current.section),
        pinned: pin || current.pinned,
      });
    },

    collapse() {
      clearTimers();
      set({ mode: "compact", pinned: false });
    },

    togglePin() {
      const pinned = !get().pinned;
      set({ pinned });
      if (!pinned && !get().hovering) scheduleCollapse();
    },

    selectSection(section) {
      set({ section });
    },

    dragEntered() {
      const { mode, draggingOut } = get();
      if (draggingOut) return;
      clearTimers();
      if (mode === "drop") return;
      if (mode !== "success") modeBeforeDrop = mode;
      set({ mode: "drop" });
    },

    dragLeft() {
      window.clearTimeout(dropLeaveTimer);
      dropLeaveTimer = window.setTimeout(() => {
        if (get().mode !== "drop") return;
        const restore = modeBeforeDrop === "expanded" && (get().pinned || get().hovering);
        set({ mode: restore ? "expanded" : "compact" });
      }, DROP_LEAVE_DELAY_MS);
    },

    dropped(message) {
      clearTimers();
      set({ mode: "success", successText: message, section: "shelf" });
      successTimer = window.setTimeout(() => {
        if (get().mode !== "success") return;
        if (get().hovering) {
          set({ mode: "expanded" });
        } else {
          get().collapse();
        }
      }, SUCCESS_MS);
    },

    setDraggingOut(dragging) {
      set({ draggingOut: dragging });
      if (!dragging && get().mode === "expanded" && !get().hovering && !get().pinned) scheduleCollapse();
    },
  };
});
