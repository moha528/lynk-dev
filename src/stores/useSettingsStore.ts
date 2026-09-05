import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";

import { DEFAULT_MODULE, type ModuleId, isModuleId } from "@/lib/modules";
import { DEFAULT_THEME, type ThemeId } from "@/lib/themes";

/**
 * Application preferences, persisted in the backend `settings` KV table.
 * Generic template baseline — modules add their own keys as needed.
 */
type Settings = {
  /** Sidebar width in px. */
  sidebarWidth: number;
  /** Active theme palette (sidebar, dialogs, main area). */
  appTheme: ThemeId;
  windowWidth: number | null;
  windowHeight: number | null;
  /** Auto-lock after N minutes of inactivity. `0` disables. */
  autoLockMinutes: number;
  /** Opt-in anonymous crash reporting (stored preference only). */
  crashReportingOptIn: boolean;
  /** Last app version seen by the user (drives the "what's new" flow). */
  lastSeenVersion: string | null;
  /** Window close (X) behavior. */
  closeBehavior: "ask" | "tray" | "minimize" | "quit";
  /** Length of the configured PIN, for auto-submit on unlock. `null` = unknown. */
  pinLength: number | null;
  /** Module shown in the main area. */
  activeModule: ModuleId;
  /** Dev Manager profile last opened. */
  devProfileId: string | null;
  /** Git Manager profile last opened. */
  gitProfileId: string | null;
};

const DEFAULTS: Settings = {
  sidebarWidth: 260,
  appTheme: DEFAULT_THEME,
  windowWidth: null,
  windowHeight: null,
  autoLockMinutes: 0,
  crashReportingOptIn: false,
  lastSeenVersion: null,
  closeBehavior: "ask",
  pinLength: null,
  activeModule: DEFAULT_MODULE,
  devProfileId: null,
  gitProfileId: null,
};

type SettingsState = Settings & {
  hydrated: boolean;
  hydrate: () => Promise<void>;
  set: <K extends keyof Settings>(key: K, value: Settings[K]) => Promise<void>;
};

const KEY_MAP: Record<keyof Settings, string> = {
  sidebarWidth: "sidebar_width",
  appTheme: "app_theme",
  windowWidth: "window_width",
  windowHeight: "window_height",
  autoLockMinutes: "auto_lock_minutes",
  crashReportingOptIn: "crash_reporting_opt_in",
  lastSeenVersion: "last_seen_version",
  closeBehavior: "close_behavior",
  pinLength: "pin_length",
  activeModule: "active_module",
  devProfileId: "dev_profile_id",
  gitProfileId: "git_profile_id",
};

export const useSettingsStore = create<SettingsState>((set) => ({
  ...DEFAULTS,
  hydrated: false,

  async hydrate() {
    try {
      const raw = await invoke<Record<string, unknown>>("get_all_settings");
      const patch: Record<string, unknown> = {};
      for (const [field, key] of Object.entries(KEY_MAP)) {
        if (!(key in raw)) continue;
        // A database written by an older build can still hold a module that no
        // longer exists (`"db"`). Skipping it keeps the default, where letting
        // it through would render an empty main area.
        if (field === "activeModule" && !isModuleId(raw[key])) continue;
        patch[field] = raw[key];
      }
      set({ ...(patch as Partial<Settings>), hydrated: true });
    } catch (e) {
      console.warn("settings hydrate:", e);
      set({ hydrated: true });
    }
  },

  async set(field, value) {
    set({ [field]: value } as Partial<SettingsState>);
    try {
      await invoke("set_setting", { key: KEY_MAP[field], value });
    } catch (e) {
      console.warn("set_setting:", e);
    }
  },
}));
