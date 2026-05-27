import { Check, Info, Keyboard, Palette, ShieldCheck, X } from "lucide-react";
import { useEffect, useState } from "react";

import { AboutSection } from "@/components/AboutSection";
import { KeybindingsSection } from "@/components/KeybindingsSection";
import { SecuritySection } from "@/components/SecuritySection";
import { TERMINAL_THEMES, type ThemeId } from "@/lib/themes";
import { cn } from "@/lib/utils";
import { useSettingsStore } from "@/stores/useSettingsStore";

type Props = {
  open: boolean;
  onClose: () => void;
};

type SectionId = "appearance" | "keybindings" | "security" | "about";

type NavEntry = { id: SectionId; label: string; icon: React.ReactNode };

const NAV: NavEntry[] = [
  { id: "appearance", label: "Apparence", icon: <Palette className="h-3.5 w-3.5" /> },
  { id: "keybindings", label: "Raccourcis", icon: <Keyboard className="h-3.5 w-3.5" /> },
  { id: "security", label: "Sécurité", icon: <ShieldCheck className="h-3.5 w-3.5" /> },
  { id: "about", label: "À propos", icon: <Info className="h-3.5 w-3.5" /> },
];

/** Centered settings modal with left navigation and a scrollable content pane. */
export function SettingsView({ open, onClose }: Props) {
  const [section, setSection] = useState<SectionId>("appearance");

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
      <button
        type="button"
        aria-label="Fermer les réglages"
        onClick={onClose}
        className="absolute inset-0 cursor-default"
      />
      <div
        className={cn(
          "relative flex h-[80vh] max-h-[700px] w-[90vw] max-w-4xl",
          "overflow-hidden rounded-xl border border-(--color-border-strong) bg-(--color-panel)",
          "shadow-2xl shadow-black/40",
        )}
      >
        <aside className="flex h-full w-[200px] shrink-0 flex-col border-r border-(--color-border) bg-(--color-bg-soft)">
          <header className="flex h-10 shrink-0 items-center px-3">
            <span className="text-xs font-semibold uppercase tracking-wider text-(--color-muted)">
              Réglages
            </span>
          </header>
          <ul className="flex-1 overflow-y-auto px-2 pb-2">
            {NAV.map((entry) => (
              <li key={entry.id}>
                <button
                  type="button"
                  onClick={() => setSection(entry.id)}
                  className={cn(
                    "group flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs transition-colors",
                    section === entry.id
                      ? "bg-(--color-panel) text-(--color-text)"
                      : "text-(--color-muted) hover:bg-(--color-panel-hover) hover:text-(--color-text-soft)",
                  )}
                >
                  <span
                    className={cn(
                      "grid h-5 w-5 shrink-0 place-items-center rounded text-(--color-muted)",
                      section === entry.id && "text-(--color-accent)",
                    )}
                  >
                    {entry.icon}
                  </span>
                  <span className="flex-1 truncate font-medium">{entry.label}</span>
                  {section === entry.id && (
                    <span className="h-1.5 w-1.5 rounded-full bg-(--color-accent)" />
                  )}
                </button>
              </li>
            ))}
          </ul>
        </aside>

        <main className="relative flex h-full min-w-0 flex-1 flex-col">
          <header className="flex h-10 shrink-0 items-center justify-between border-b border-(--color-border) px-4">
            <h2 className="text-sm font-semibold text-(--color-text)">
              {NAV.find((n) => n.id === section)?.label}
            </h2>
            <button
              type="button"
              aria-label="Fermer"
              onClick={onClose}
              className="rounded-md p-1 text-(--color-muted) hover:bg-(--color-panel-hover) hover:text-(--color-text)"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </header>
          <div className="min-h-0 flex-1 overflow-y-auto p-5">
            <SectionContent section={section} />
          </div>
        </main>
      </div>
    </div>
  );
}

function SectionContent({ section }: { section: SectionId }) {
  const appTheme = useSettingsStore((s) => s.appTheme);
  const closeBehavior = useSettingsStore((s) => s.closeBehavior);
  const setSetting = useSettingsStore((s) => s.set);

  switch (section) {
    case "appearance":
      return (
        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-2">
            <p className="text-xs text-(--color-muted)">
              Palette utilisée par la sidebar, les dialogs et la zone principale.
            </p>
            <ThemeGrid selectedId={appTheme} onSelect={(id) => setSetting("appTheme", id)} />
          </div>
          <Choice
            label="À la fermeture de la fenêtre"
            description="Que faire quand tu cliques sur la croix. La zone de notification garde l'app en arrière-plan."
            value={closeBehavior}
            options={[
              { value: "ask", label: "Demander", hint: "défaut" },
              { value: "tray", label: "Zone de notif." },
              { value: "minimize", label: "Réduire" },
              { value: "quit", label: "Quitter" },
            ]}
            onChange={(v) => setSetting("closeBehavior", v as "ask" | "tray" | "minimize" | "quit")}
          />
        </div>
      );
    case "keybindings":
      return <KeybindingsSection />;
    case "security":
      return <SecuritySection />;
    case "about":
      return <AboutSection />;
  }
}

function Choice({
  label,
  description,
  value,
  options,
  onChange,
}: {
  label: string;
  description?: string;
  value: string;
  options: Array<{ value: string; label: string; hint?: string }>;
  onChange: (v: string) => void;
}) {
  return (
    <div className="flex flex-col gap-1.5 rounded-md border border-(--color-border) bg-(--color-bg-soft) p-3">
      <div className="flex flex-col">
        <span className="text-xs font-medium text-(--color-text)">{label}</span>
        {description && <span className="text-[10px] text-(--color-muted)">{description}</span>}
      </div>
      <div className="flex gap-1.5">
        {options.map((o) => (
          <button
            key={o.value}
            type="button"
            onClick={() => onChange(o.value)}
            className={cn(
              "flex flex-1 items-center justify-center gap-1 rounded-md border px-2 py-1.5 text-[11px] transition-colors",
              value === o.value
                ? "border-(--color-accent) bg-(--color-accent-bg)/30 text-(--color-text)"
                : "border-(--color-border) bg-(--color-panel) text-(--color-muted) hover:bg-(--color-panel-hover) hover:text-(--color-text)",
            )}
          >
            {o.label}
            {o.hint && <span className="text-[10px] text-(--color-muted-soft)">· {o.hint}</span>}
          </button>
        ))}
      </div>
    </div>
  );
}

function ThemeGrid({
  selectedId,
  onSelect,
}: {
  selectedId: ThemeId;
  onSelect: (id: ThemeId) => void;
}) {
  return (
    <div className="grid grid-cols-2 gap-1.5">
      {(Object.entries(TERMINAL_THEMES) as [ThemeId, (typeof TERMINAL_THEMES)[ThemeId]][]).map(
        ([id, t]) => {
          const selected = selectedId === id;
          return (
            <button
              key={id}
              type="button"
              onClick={() => onSelect(id)}
              title={t.name}
              className={cn(
                "group relative flex items-center gap-2 overflow-hidden rounded-md border px-2 py-2 text-left transition-all",
                selected
                  ? "border-(--color-accent) bg-(--color-accent-bg)/30 ring-1 ring-(--color-accent)/30"
                  : "border-(--color-border) bg-(--color-bg-soft) hover:border-(--color-border-strong) hover:bg-(--color-panel-hover)",
              )}
            >
              <AppPreview palette={t.app} />
              <span className="min-w-0 flex-1 truncate text-xs font-medium">{t.name}</span>
              {selected && <Check className="h-3 w-3 shrink-0 text-(--color-accent)" />}
            </button>
          );
        },
      )}
    </div>
  );
}

function AppPreview({ palette }: { palette: Record<string, string> }) {
  return (
    <div
      className="flex h-6 w-6 shrink-0 overflow-hidden rounded-sm border"
      style={{
        borderColor: palette["--color-border-strong"],
        background: palette["--color-bg"],
      }}
      aria-hidden
    >
      <div className="w-1/3" style={{ background: palette["--color-bg-soft"] }} />
      <div className="flex flex-1 items-end justify-end p-0.5">
        <span
          className="block h-1.5 w-1.5 rounded-full"
          style={{ background: palette["--color-accent"] }}
        />
      </div>
    </div>
  );
}
