import { useEffect } from "react";

import { type ActionId, eventToAccel, normalizeAccel } from "@/lib/keybindings";
import { useKeybindingsStore } from "@/stores/useKeybindingsStore";

export type ShortcutHandlers = Partial<Record<ActionId, () => void>>;

/**
 * Subscribe a window-level keydown listener that dispatches each event to
 * the matching action handler in `handlers`, based on the current bindings.
 *
 * Inputs and contenteditable fields get a pass — typing into a search input
 * shouldn't fire a global shortcut. We make a single exception for
 * `open-command-palette` / `open-snippets` / `open-command-history` so the
 * user can summon those from anywhere, including while editing.
 *
 * The handler set is read live each event via the store so re-binding is
 * effective without re-registering the listener.
 */
export function useShortcuts(handlers: ShortcutHandlers): void {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const accel = eventToAccel(e);
      if (!accel) return;
      const norm = normalizeAccel(accel);
      const bindings = useKeybindingsStore.getState().bindings;
      const action = (Object.entries(bindings) as [ActionId, string][]).find(
        ([, a]) => normalizeAccel(a) === norm,
      )?.[0];
      if (!action) return;
      const handler = handlers[action];
      if (!handler) return;

      const target = e.target as HTMLElement | null;
      const inEditable = isEditableTarget(target);
      // Allow these to fire from inputs — they're the user's escape hatch.
      const fireFromInputs: ActionId[] = ["open-command-palette", "open-settings"];
      if (inEditable && !fireFromInputs.includes(action)) return;

      e.preventDefault();
      e.stopPropagation();
      handler();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [handlers]);
}

function isEditableTarget(el: HTMLElement | null): boolean {
  if (!el) return false;
  const tag = el.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
  if (el.isContentEditable) return true;
  return false;
}
