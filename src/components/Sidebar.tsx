import { Boxes, Database, GitBranch } from "lucide-react";

type SidebarProps = {
  width: number;
};

/**
 * Left navigation rail. Template placeholder: the future Lynk Dev modules
 * (Git Manager, Dev Manager, DB Explorer) plug their entries in here.
 */
export function Sidebar({ width }: SidebarProps) {
  const modules = [
    { id: "git", label: "Git Manager", icon: <GitBranch className="h-4 w-4" /> },
    { id: "dev", label: "Dev Manager", icon: <Boxes className="h-4 w-4" /> },
    { id: "db", label: "DB Explorer", icon: <Database className="h-4 w-4" /> },
  ];

  return (
    <aside
      style={{ width }}
      className="flex shrink-0 flex-col gap-1 border-r border-(--color-border) bg-(--color-bg-soft) p-2"
    >
      <p className="px-2 pb-1 pt-2 text-[10px] font-semibold uppercase tracking-wider text-(--color-muted)">
        Modules
      </p>
      {modules.map((m) => (
        <button
          key={m.id}
          type="button"
          disabled
          className="flex items-center gap-2.5 rounded-md px-2.5 py-2 text-left text-xs text-(--color-muted) opacity-60"
          title="À venir"
        >
          <span className="text-(--color-muted)">{m.icon}</span>
          <span className="flex-1 truncate font-medium">{m.label}</span>
          <span className="rounded bg-(--color-panel) px-1.5 py-0.5 text-[9px] uppercase tracking-wide text-(--color-muted-soft)">
            soon
          </span>
        </button>
      ))}
    </aside>
  );
}
