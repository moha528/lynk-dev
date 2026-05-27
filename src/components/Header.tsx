import { Command, Settings as SettingsIcon } from "lucide-react";

type Props = {
  onOpenSettings: () => void;
  onOpenPalette: () => void;
};

/**
 * App header rendered just below the native OS title bar.
 * Hosts the brand and the global actions (command palette, settings).
 */
export function Header({ onOpenSettings, onOpenPalette }: Props) {
  return (
    <header className="flex h-11 shrink-0 items-center justify-between border-b border-(--color-border) bg-(--color-panel) px-3">
      <div className="flex items-center gap-2.5">
        <img
          src="/logo-mark.png"
          alt="Lynk Dev"
          className="h-7 w-7 select-none"
          draggable={false}
        />
        <div className="flex flex-col leading-tight">
          <span className="text-sm font-semibold tracking-tight text-(--color-text)">Lynk Dev</span>
          <span className="text-[10px] uppercase tracking-wider text-(--color-muted)">
            Dev tools
          </span>
        </div>
      </div>

      <div className="flex items-center gap-1">
        <button
          type="button"
          onClick={onOpenPalette}
          title="Palette de commandes  ·  Ctrl+K"
          className="inline-flex items-center gap-2 rounded-md border border-(--color-border) bg-(--color-bg-soft) px-2.5 py-1 text-xs text-(--color-muted) transition-colors hover:bg-(--color-panel-hover) hover:text-(--color-text)"
        >
          <Command className="h-3.5 w-3.5" />
          <span>Rechercher</span>
          <span className="flex items-center gap-0.5 text-(--color-muted-soft)">
            <kbd className="rounded border border-(--color-border) bg-(--color-bg) px-1 font-mono text-[10px]">
              Ctrl
            </kbd>
            <kbd className="rounded border border-(--color-border) bg-(--color-bg) px-1 font-mono text-[10px]">
              K
            </kbd>
          </span>
        </button>
        <button
          type="button"
          aria-label="Réglages"
          onClick={onOpenSettings}
          className="rounded-md p-1.5 text-(--color-muted) transition-colors hover:bg-(--color-panel-hover) hover:text-(--color-text)"
        >
          <SettingsIcon className="h-4 w-4" />
        </button>
      </div>
    </header>
  );
}
