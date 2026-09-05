import { Palette, Search, Settings as SettingsIcon } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { TERMINAL_THEMES, type ThemeId } from "@/lib/themes";
import { cn } from "@/lib/utils";
import { useSettingsStore } from "@/stores/useSettingsStore";

type Props = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onOpenSettings: () => void;
};

type Command = {
  id: string;
  label: string;
  hint?: string;
  icon: React.ReactNode;
  run: () => void;
};

/**
 * Lightweight command palette (Ctrl+K). Template baseline: open settings +
 * switch theme. Each module registers its own commands here.
 */
export function CommandPalette({ open, onOpenChange, onOpenSettings }: Props) {
  const [query, setQuery] = useState("");
  const setSetting = useSettingsStore((s) => s.set);

  useEffect(() => {
    if (open) setQuery("");
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onOpenChange(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onOpenChange]);

  const commands = useMemo<Command[]>(() => {
    const base: Command[] = [
      {
        id: "settings",
        label: "Ouvrir les réglages",
        hint: "Ctrl+,",
        icon: <SettingsIcon className="h-3.5 w-3.5" />,
        run: () => {
          onOpenChange(false);
          onOpenSettings();
        },
      },
    ];
    const themes = (Object.entries(TERMINAL_THEMES) as [ThemeId, { name: string }][]).map(
      ([id, t]) => ({
        id: `theme-${id}`,
        label: `Thème : ${t.name}`,
        icon: <Palette className="h-3.5 w-3.5" />,
        run: () => {
          void setSetting("appTheme", id);
          onOpenChange(false);
        },
      }),
    );
    return [...base, ...themes];
  }, [onOpenChange, onOpenSettings, setSetting]);

  const filtered = commands.filter((c) => c.label.toLowerCase().includes(query.toLowerCase()));

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-[90] flex items-start justify-center p-4 pt-[12vh]">
      <button
        type="button"
        aria-label="Fermer"
        onClick={() => onOpenChange(false)}
        className="absolute inset-0 cursor-default bg-black/60 backdrop-blur-sm"
      />
      <div className="relative z-10 w-full max-w-lg overflow-hidden rounded-xl border border-(--color-border-strong) bg-(--color-panel) shadow-2xl">
        <div className="flex items-center gap-2 border-b border-(--color-border) px-3">
          <Search className="h-4 w-4 text-(--color-muted)" />
          <input
            // biome-ignore lint/a11y/noAutofocus: palette opens on user intent
            autoFocus
            value={query}
            onChange={(e) => setQuery(e.currentTarget.value)}
            placeholder="Rechercher une commande…"
            className="h-11 flex-1 bg-transparent text-sm text-(--color-text) outline-none placeholder:text-(--color-muted)"
          />
        </div>
        <ul className="max-h-80 overflow-y-auto p-1.5">
          {filtered.length === 0 ? (
            <li className="px-3 py-6 text-center text-xs text-(--color-muted)">Aucune commande.</li>
          ) : (
            filtered.map((c) => (
              <li key={c.id}>
                <button
                  type="button"
                  onClick={c.run}
                  className={cn(
                    "flex w-full items-center gap-2.5 rounded-md px-3 py-2 text-left text-sm",
                    "text-(--color-text-soft) transition-colors hover:bg-(--color-panel-hover) hover:text-(--color-text)",
                  )}
                >
                  <span className="text-(--color-muted)">{c.icon}</span>
                  <span className="flex-1 truncate">{c.label}</span>
                  {c.hint && (
                    <span className="font-mono text-[10px] text-(--color-muted-soft)">
                      {c.hint}
                    </span>
                  )}
                </button>
              </li>
            ))
          )}
        </ul>
      </div>
    </div>
  );
}
