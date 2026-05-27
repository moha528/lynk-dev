import { useEffect, useState } from "react";

export type CloseAction = "tray" | "minimize" | "quit";

const OPTIONS: { action: CloseAction; label: string; desc: string }[] = [
  {
    action: "tray",
    label: "Réduire dans la zone de notification",
    desc: "L'app continue de tourner en arrière-plan (icône près de l'horloge).",
  },
  {
    action: "minimize",
    label: "Réduire la fenêtre",
    desc: "Minimise dans la barre des tâches.",
  },
  {
    action: "quit",
    label: "Quitter Lynk Dev",
    desc: "Ferme complètement l'application.",
  },
];

/**
 * Demandé au clic sur la croix quand le réglage est « toujours demander ».
 * L'utilisateur choisit l'action et peut cocher « se souvenir » pour la
 * définir comme comportement par défaut.
 */
export function CloseActionDialog({
  open,
  onAction,
  onCancel,
}: {
  open: boolean;
  onAction: (action: CloseAction, remember: boolean) => void;
  onCancel: () => void;
}) {
  const [remember, setRemember] = useState(false);

  useEffect(() => {
    if (open) setRemember(false);
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onCancel]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center p-4">
      <button
        type="button"
        aria-label="Annuler"
        onClick={onCancel}
        className="absolute inset-0 cursor-default bg-black/60 backdrop-blur-sm"
      />
      <div className="relative z-10 w-full max-w-sm overflow-hidden rounded-2xl border border-(--color-border) bg-(--color-panel) shadow-2xl">
        <div className="flex items-center gap-3 border-b border-(--color-border) px-5 py-4">
          <img src="/logo-mark.png" alt="" className="h-7 w-7 select-none" draggable={false} />
          <h2 className="text-sm font-semibold text-(--color-text)">Fermer Lynk Dev ?</h2>
        </div>

        <div className="flex flex-col gap-1.5 p-3">
          {OPTIONS.map((o) => (
            <button
              key={o.action}
              type="button"
              onClick={() => onAction(o.action, remember)}
              className="flex flex-col rounded-lg border border-(--color-border) bg-(--color-bg-soft) px-3 py-2.5 text-left transition-colors hover:border-(--color-accent) hover:bg-(--color-accent-bg)/20"
            >
              <span className="text-sm font-medium text-(--color-text)">{o.label}</span>
              <span className="text-[11px] text-(--color-muted)">{o.desc}</span>
            </button>
          ))}
        </div>

        <div className="flex items-center justify-between gap-3 border-t border-(--color-border) px-5 py-3">
          <label className="flex cursor-pointer items-center gap-2 text-xs text-(--color-muted)">
            <input
              type="checkbox"
              checked={remember}
              onChange={(e) => setRemember(e.target.checked)}
              className="h-3.5 w-3.5 accent-(--color-accent)"
            />
            Se souvenir de mon choix
          </label>
          <button
            type="button"
            onClick={onCancel}
            className="rounded-md px-3 py-1.5 text-xs text-(--color-muted) transition-colors hover:bg-(--color-panel-hover) hover:text-(--color-text)"
          >
            Annuler
          </button>
        </div>
      </div>
    </div>
  );
}
