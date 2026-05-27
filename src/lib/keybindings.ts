/**
 * Keyboard shortcut registry.
 *
 * Actions are identified by string ids and bound to a single user-editable
 * accelerator. Bindings persist via the keybindings store. At runtime
 * [`useShortcuts`] registers a `keydown` listener that resolves each event to
 * a binding and fires the matching handler.
 *
 * Accelerator syntax: `Ctrl+Shift+S`, `Cmd+K`, `Alt+Enter`, `Ctrl+/`. The
 * parser is case-insensitive on modifier names. `Ctrl` and `Cmd` (metaKey)
 * fire the same actions; the label stays "Ctrl" for simplicity.
 *
 * Template baseline: two generic actions. Each module appends its own.
 */

export type ActionId = "open-command-palette" | "open-settings";

export type ActionDefinition = {
  id: ActionId;
  label: string;
  category: "Navigation" | "Général";
  defaultAccel: string;
};

export const ACTIONS: ActionDefinition[] = [
  {
    id: "open-command-palette",
    label: "Palette de commandes",
    category: "Navigation",
    defaultAccel: "Ctrl+K",
  },
  {
    id: "open-settings",
    label: "Ouvrir les réglages",
    category: "Général",
    defaultAccel: "Ctrl+,",
  },
];

export const DEFAULT_BINDINGS: Record<ActionId, string> = Object.fromEntries(
  ACTIONS.map((a) => [a.id, a.defaultAccel]),
) as Record<ActionId, string>;

export type Bindings = Record<ActionId, string>;

/**
 * Resolve a `KeyboardEvent` to its canonical accelerator string. Returns
 * `null` when `event.key` is a bare modifier (no real key pressed yet).
 */
export function eventToAccel(e: KeyboardEvent): string | null {
  const key = normalizeKey(e.key);
  if (key === null) return null;
  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  parts.push(key);
  return parts.join("+");
}

function normalizeKey(key: string): string | null {
  if (["Control", "Meta", "Alt", "Shift"].includes(key)) return null;
  if (key.length === 1) return key.toUpperCase();
  if (key === " ") return "Space";
  return key;
}

/**
 * Normalize a user-supplied accelerator string to the canonical
 * `Ctrl+Shift+Key` ordering so order differences compare equal.
 */
export function normalizeAccel(accel: string): string {
  const parts = accel.split("+").map((p) => p.trim());
  if (parts.length === 0) return "";
  const last = parts.pop() ?? "";
  const mods = new Set(parts.map((p) => p.toLowerCase()));
  const out: string[] = [];
  if (mods.has("ctrl") || mods.has("cmd") || mods.has("meta")) out.push("Ctrl");
  if (mods.has("alt") || mods.has("option")) out.push("Alt");
  if (mods.has("shift")) out.push("Shift");
  out.push(last.length === 1 ? last.toUpperCase() : last);
  return out.join("+");
}

/**
 * Return the list of `(accel, actions)` pairs that collide on the same
 * accelerator. Used by the Settings UI to warn the user.
 */
export function findConflicts(bindings: Bindings): Array<{
  accel: string;
  actions: ActionId[];
}> {
  const byAccel = new Map<string, ActionId[]>();
  for (const [id, accel] of Object.entries(bindings) as [ActionId, string][]) {
    if (!accel) continue;
    const norm = normalizeAccel(accel);
    if (!byAccel.has(norm)) byAccel.set(norm, []);
    byAccel.get(norm)?.push(id);
  }
  const out: Array<{ accel: string; actions: ActionId[] }> = [];
  for (const [accel, actions] of byAccel) {
    if (actions.length > 1) out.push({ accel, actions });
  }
  return out;
}
