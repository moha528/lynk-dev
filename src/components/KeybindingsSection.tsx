import { AlertTriangle, Check, RotateCcw } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import {
  ACTIONS,
  type ActionDefinition,
  type ActionId,
  type Bindings,
  DEFAULT_BINDINGS,
  eventToAccel,
  findConflicts,
} from "@/lib/keybindings";
import { cn } from "@/lib/utils";
import { useKeybindingsStore } from "@/stores/useKeybindingsStore";

import { Button } from "./ui/Button";

/**
 * Settings section listing every action and its bound accelerator (P4-T07).
 *
 * Click an accelerator to enter "recording mode" — the next non-modifier key
 * combination is captured and saved. The recorder filters out bare modifiers
 * so the user can hold them while choosing the final key.
 *
 * Conflicts are shown as a yellow banner at the top. They don't block save —
 * the dispatcher just fires whichever action it finds first in the registry.
 */
export function KeybindingsSection() {
  const bindings = useKeybindingsStore((s) => s.bindings);
  const setBinding = useKeybindingsStore((s) => s.setBinding);
  const resetAll = useKeybindingsStore((s) => s.resetAll);
  const [recording, setRecording] = useState<ActionId | null>(null);

  const conflicts = useMemo(() => findConflicts(bindings), [bindings]);

  const grouped = useMemo(() => {
    const map = new Map<string, ActionDefinition[]>();
    for (const a of ACTIONS) {
      if (!map.has(a.category)) map.set(a.category, []);
      map.get(a.category)?.push(a);
    }
    return Array.from(map.entries());
  }, []);

  return (
    <div className="flex flex-col gap-4">
      <header className="flex items-start justify-between gap-3">
        <p className="text-xs text-(--color-muted)">
          Clic sur un raccourci pour le ré-enregistrer. Les modifications s'appliquent
          immédiatement.
        </p>
        <Button
          size="sm"
          variant="outline"
          className="gap-1.5 text-[11px]"
          onClick={() => void resetAll()}
        >
          <RotateCcw className="h-3 w-3" />
          Réinitialiser
        </Button>
      </header>

      {conflicts.length > 0 && (
        <div className="flex items-start gap-2 rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-[11px] text-amber-200">
          <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
          <div className="flex flex-col gap-1">
            <span>
              {conflicts.length} conflit{conflicts.length > 1 ? "s" : ""} de raccourci :
            </span>
            <ul className="list-disc pl-4">
              {conflicts.map((c) => (
                <li key={c.accel}>
                  <code className="font-mono text-amber-100">{c.accel}</code> :{" "}
                  {c.actions.map((id) => ACTIONS.find((a) => a.id === id)?.label ?? id).join(" · ")}
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}

      <div className="flex flex-col gap-3">
        {grouped.map(([category, actions]) => (
          <section key={category} className="flex flex-col gap-1">
            <h3 className="text-[10px] font-semibold uppercase tracking-wider text-(--color-muted)">
              {category}
            </h3>
            <ul className="flex flex-col gap-1">
              {actions.map((a) => (
                <BindingRow
                  key={a.id}
                  action={a}
                  bindings={bindings}
                  recording={recording === a.id}
                  onStart={() => setRecording(a.id)}
                  onCancel={() => setRecording(null)}
                  onCommit={(accel) => {
                    void setBinding(a.id, accel);
                    setRecording(null);
                  }}
                />
              ))}
            </ul>
          </section>
        ))}
      </div>
    </div>
  );
}

function BindingRow({
  action,
  bindings,
  recording,
  onStart,
  onCancel,
  onCommit,
}: {
  action: ActionDefinition;
  bindings: Bindings;
  recording: boolean;
  onStart: () => void;
  onCancel: () => void;
  onCommit: (accel: string) => void;
}) {
  const accel = bindings[action.id];
  const isDefault = accel === DEFAULT_BINDINGS[action.id];
  const buttonRef = useRef<HTMLButtonElement | null>(null);

  // While recording, capture the next non-modifier key as the new binding.
  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        onCancel();
        return;
      }
      const a = eventToAccel(e);
      if (a) onCommit(a);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, onCancel, onCommit]);

  return (
    <li className="flex items-center justify-between gap-2 rounded-md border border-(--color-border) bg-(--color-bg-soft) px-3 py-1.5">
      <span className="text-xs text-(--color-text)">{action.label}</span>
      <div className="flex items-center gap-1.5">
        {!isDefault && (
          <button
            type="button"
            onClick={() => onCommit(DEFAULT_BINDINGS[action.id])}
            title="Restaurer la valeur par défaut"
            aria-label="Restaurer la valeur par défaut"
            className="rounded p-0.5 text-(--color-muted) hover:bg-(--color-panel-hover) hover:text-(--color-text)"
          >
            <RotateCcw className="h-3 w-3" />
          </button>
        )}
        <button
          ref={buttonRef}
          type="button"
          onClick={() => (recording ? onCancel() : onStart())}
          className={cn(
            "min-w-[110px] rounded-md border px-2 py-1 font-mono text-[11px]",
            recording
              ? "animate-pulse border-(--color-accent) bg-(--color-accent-bg)/30 text-(--color-accent)"
              : "border-(--color-border) bg-(--color-panel) text-(--color-text-soft) hover:border-(--color-border-strong) hover:text-(--color-text)",
          )}
        >
          {recording ? "Appuie sur une touche…" : accel || "—"}
        </button>
        {!recording && !isDefault && (
          <Check className="h-3 w-3 text-(--color-accent)" aria-label="Personnalisé" />
        )}
      </div>
    </li>
  );
}
