import { MODULES, type ModuleId } from "@/lib/modules";
import { cn } from "@/lib/utils";

type SidebarProps = {
  width: number;
  active: ModuleId;
  onSelect: (id: ModuleId) => void;
};

/** Left navigation rail: one entry per Lynk Dev module. */
export function Sidebar({ width, active, onSelect }: SidebarProps) {
  return (
    <aside
      style={{ width }}
      className="flex shrink-0 flex-col gap-1 border-r border-(--color-border) bg-(--color-bg-soft) p-2"
    >
      <p className="px-2 pb-1 pt-2 text-[10px] font-semibold uppercase tracking-wider text-(--color-muted)">
        Modules
      </p>
      {MODULES.map((m) => {
        const Icon = m.icon;
        const isActive = m.id === active;
        return (
          <button
            key={m.id}
            type="button"
            onClick={() => onSelect(m.id)}
            aria-current={isActive ? "page" : undefined}
            className={cn(
              "flex items-center gap-2.5 rounded-md px-2.5 py-2 text-left text-xs transition-colors",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent)",
              isActive
                ? "bg-(--color-accent-bg) font-medium text-(--color-text)"
                : "text-(--color-muted) hover:bg-(--color-panel-hover) hover:text-(--color-text-soft)",
            )}
          >
            <Icon
              className={cn("h-4 w-4", isActive ? "text-(--color-accent)" : "text-(--color-muted)")}
            />
            <span className="flex-1 truncate">{m.label}</span>
          </button>
        );
      })}
    </aside>
  );
}
