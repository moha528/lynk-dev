import { useCallback, useEffect, useMemo, useState } from "react";

import { getCurrentWindow } from "@tauri-apps/api/window";
import { exit } from "@tauri-apps/plugin-process";
import { toast } from "sonner";

import { withToast } from "@/lib/feedback";
import { checkForUpdate, installUpdate } from "@/lib/updater";
import { useShortcuts } from "@/lib/useShortcuts";
import { DevManagerView } from "@/modules/dev/DevManagerView";
import { GitManagerView } from "@/modules/git/GitManagerView";
import { useKeybindingsStore } from "@/stores/useKeybindingsStore";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { useVaultStore } from "@/stores/useVaultStore";
import { SettingsView } from "@/views/SettingsView";

import { type CloseAction, CloseActionDialog } from "./CloseActionDialog";
import { CommandPalette } from "./CommandPalette";
import { ErrorBoundary } from "./ErrorBoundary";
import { Header } from "./Header";
import { Sidebar } from "./Sidebar";
import { SidebarResizer } from "./SidebarResizer";
import { UnlockOverlay } from "./UnlockOverlay";

export function MainLayout() {
  const sidebarWidth = useSettingsStore((s) => s.sidebarWidth);
  const activeModule = useSettingsStore((s) => s.activeModule);
  const autoLockMinutes = useSettingsStore((s) => s.autoLockMinutes);
  const setSetting = useSettingsStore((s) => s.set);
  const hydrate = useSettingsStore((s) => s.hydrate);

  const [settingsOpen, setSettingsOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [closeAsk, setCloseAsk] = useState(false);

  // Startup hydration: settings, keybindings, vault (locks the app if a PIN
  // is configured), and a silent update check.
  useEffect(() => {
    void hydrate();
    void useKeybindingsStore.getState().hydrate();
    void useVaultStore.getState().hydrate();

    void checkForUpdate()
      .then((info) => {
        if (!info) return;
        toast(`Mise à jour disponible — v${info.version}`, {
          description: "Une nouvelle version de Lynk Dev est prête.",
          duration: Number.POSITIVE_INFINITY,
          action: {
            label: "Installer",
            onClick: () => {
              void withToast(installUpdate(info), {
                loading: "Téléchargement et installation…",
                success: "Mise à jour installée — redémarrage…",
              });
            },
          },
        });
      })
      .catch((e) => console.debug("update check (silencieux):", e));
  }, [hydrate]);

  // Configurable shortcut dispatch.
  const shortcutHandlers = useMemo(
    () => ({
      "open-command-palette": () => setPaletteOpen((v) => !v),
      "open-settings": () => setSettingsOpen((v) => !v),
    }),
    [],
  );
  useShortcuts(shortcutHandlers);

  // Auto-lock after inactivity. No-op when disabled or no PIN configured.
  useEffect(() => {
    if (autoLockMinutes <= 0) return;
    let lastActivity = Date.now();
    const onActivity = () => {
      lastActivity = Date.now();
    };
    const events: (keyof DocumentEventMap)[] = ["keydown", "mousedown", "mousemove", "scroll"];
    for (const e of events) document.addEventListener(e, onActivity, { passive: true });
    const id = window.setInterval(() => {
      const v = useVaultStore.getState();
      if (!v.hasMaster || v.locked) return;
      if (Date.now() - lastActivity >= autoLockMinutes * 60_000) void v.lock();
    }, 30_000);
    return () => {
      window.clearInterval(id);
      for (const e of events) document.removeEventListener(e, onActivity);
    };
  }, [autoLockMinutes]);

  const applyCloseAction = useCallback(async (action: CloseAction) => {
    const win = getCurrentWindow();
    if (action === "tray") await win.hide();
    else if (action === "minimize") await win.minimize();
    else await exit(0);
  }, []);

  // Intercept the window close (X) button per the user's preference.
  useEffect(() => {
    const win = getCurrentWindow();
    let unlisten: (() => void) | undefined;
    void win
      .onCloseRequested(async (event) => {
        const behavior = useSettingsStore.getState().closeBehavior;
        event.preventDefault();
        if (behavior === "ask") setCloseAsk(true);
        else await applyCloseAction(behavior);
      })
      .then((u) => {
        unlisten = u;
      });
    return () => unlisten?.();
  }, [applyCloseAction]);

  const onCloseChoice = useCallback(
    (action: CloseAction, remember: boolean) => {
      setCloseAsk(false);
      if (remember) void setSetting("closeBehavior", action);
      void applyCloseAction(action);
    },
    [applyCloseAction, setSetting],
  );

  return (
    <div className="flex h-screen w-screen flex-col bg-(--color-bg) text-(--color-text)">
      <Header
        onOpenSettings={() => setSettingsOpen(true)}
        onOpenPalette={() => setPaletteOpen(true)}
      />

      <div className="flex min-h-0 flex-1">
        <Sidebar
          width={sidebarWidth}
          active={activeModule}
          onSelect={(id) => setSetting("activeModule", id)}
        />
        <SidebarResizer onResize={(w) => setSetting("sidebarWidth", w)} />

        <main className="flex min-w-0 flex-1 flex-col bg-(--color-bg)">
          {/* Keyed on the module so switching away clears a previous crash. */}
          <ErrorBoundary key={activeModule}>
            {activeModule === "git" ? <GitManagerView /> : <DevManagerView />}
          </ErrorBoundary>
        </main>
      </div>

      <SettingsView open={settingsOpen} onClose={() => setSettingsOpen(false)} />
      <CommandPalette
        open={paletteOpen}
        onOpenChange={setPaletteOpen}
        onOpenSettings={() => setSettingsOpen(true)}
      />
      <CloseActionDialog
        open={closeAsk}
        onAction={onCloseChoice}
        onCancel={() => setCloseAsk(false)}
      />
      <UnlockOverlay />
    </div>
  );
}
