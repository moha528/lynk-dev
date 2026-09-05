import { Boxes, GitBranch } from "lucide-react";
import type { ComponentType } from "react";

/**
 * The modules Lynk Dev ships.
 *
 * DB Explorer was dropped on 2026-09-05 (decision D8 in `tasks.md`) — it is not
 * a pending item, so do not add it back without revisiting that decision.
 */
export const MODULES = [
  { id: "git", label: "Git Manager", icon: GitBranch },
  { id: "dev", label: "Dev Manager", icon: Boxes },
] as const satisfies readonly {
  id: string;
  label: string;
  icon: ComponentType<{ className?: string }>;
}[];

export type ModuleId = (typeof MODULES)[number]["id"];

export const DEFAULT_MODULE: ModuleId = "git";

/**
 * Narrows an unknown value to a known module id.
 *
 * Needed because the active module is persisted: a database written by an older
 * build can still hold `"db"`, which would otherwise render nothing at all.
 */
export function isModuleId(value: unknown): value is ModuleId {
  return typeof value === "string" && MODULES.some((m) => m.id === value);
}
