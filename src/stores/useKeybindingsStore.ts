import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";

import { type ActionId, type Bindings, DEFAULT_BINDINGS, normalizeAccel } from "@/lib/keybindings";

const SETTING_KEY = "keybindings";

type KeybindingsState = {
  bindings: Bindings;
  hydrated: boolean;
  hydrate: () => Promise<void>;
  setBinding: (id: ActionId, accel: string) => Promise<void>;
  resetAll: () => Promise<void>;
};

export const useKeybindingsStore = create<KeybindingsState>((set, get) => ({
  bindings: { ...DEFAULT_BINDINGS },
  hydrated: false,

  async hydrate() {
    try {
      const raw = await invoke<Record<string, unknown>>("get_all_settings");
      const stored = raw[SETTING_KEY];
      if (stored && typeof stored === "object") {
        const merged: Bindings = { ...DEFAULT_BINDINGS };
        for (const [id, accel] of Object.entries(stored as Record<string, unknown>)) {
          if (typeof accel === "string" && id in DEFAULT_BINDINGS) {
            merged[id as ActionId] = normalizeAccel(accel);
          }
        }
        set({ bindings: merged });
      }
    } catch (e) {
      console.warn("keybindings hydrate:", e);
    } finally {
      set({ hydrated: true });
    }
  },

  async setBinding(id, accel) {
    const next = { ...get().bindings, [id]: normalizeAccel(accel) };
    set({ bindings: next });
    try {
      await invoke("set_setting", { key: SETTING_KEY, value: next });
    } catch (e) {
      console.warn("set keybinding:", e);
    }
  },

  async resetAll() {
    const next = { ...DEFAULT_BINDINGS };
    set({ bindings: next });
    try {
      await invoke("set_setting", { key: SETTING_KEY, value: next });
    } catch (e) {
      console.warn("reset keybindings:", e);
    }
  },
}));
