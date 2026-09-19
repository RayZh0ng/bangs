import { create } from "zustand";

import { native, type Activity } from "../lib/native";

interface ActivityStore {
  items: Activity[];
  update(items: Activity[]): void;
  open(item: Activity): Promise<void>;
}

export const useActivities = create<ActivityStore>((set) => ({
  items: [],
  update(items) {
    set({ items });
  },
  async open(item) {
    if (!item.url) return;
    await native.activityOpen(item.id).catch(() => {});
  },
}));

/** How recently a row must have been written to be what the panel opens on. */
const FRESH_MS = 60_000;

/** Something is happening right now, as opposed to a row parked on the notch. */
export function hasFreshActivity(items: Activity[], now: number): boolean {
  return items.some((item) => now - item.updatedAt * 1000 < FRESH_MS);
}
