import { ChevronDown, ChevronRight, Plus, Search } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { Checkbox } from "@/components/ui/Checkbox";
import { Input } from "@/components/ui/Input";
import { cn } from "@/lib/utils";

import { STATUS_LABEL, TONE_TEXT, TYPE_LABEL, formatUptime, statusTone } from "../status";
import type { ServiceRuntime, ServiceStatus } from "../types";
import { StatusDot } from "./StatusDot";

type Props = {
  runtimes: ServiceRuntime[];
  selectedId: string | null;
  onSelect: (serviceId: string) => void;
  checked: Set<string>;
  onCheck: (serviceId: string, value: boolean) => void;
  onCheckMany: (serviceIds: string[], value: boolean) => void;
  onAdd: () => void;
};

const FILTERS: { id: "all" | ServiceStatus; label: string }[] = [
  { id: "all", label: "Tous" },
  { id: "running", label: STATUS_LABEL.running },
  { id: "stopped", label: STATUS_LABEL.stopped },
  { id: "error", label: STATUS_LABEL.error },
];

const UNGROUPED = "__ungrouped__";

/**
 * Liste des services : recherche, filtre d'état, groupes repliables et
 * sélection multiple.
 *
 * La sélection multiple est ce qui manquait le plus à l'écran d'origine : sur
 * douze microservices, agir sur un sous-ensemble se faisait service par
 * service.
 */
export function ServiceList({
  runtimes,
  selectedId,
  onSelect,
  checked,
  onCheck,
  onCheckMany,
  onAdd,
}: Props) {
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<"all" | ServiceStatus>("all");
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  // La valeur n'est jamais lue : seul le rendu qu'elle déclenche compte.
  const [, setTick] = useState(0);

  // Rafraîchit les durées affichées sans toucher au store.
  useEffect(() => {
    const id = window.setInterval(() => setTick((t) => t + 1), 1_000);
    return () => window.clearInterval(id);
  }, []);

  const visible = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return runtimes.filter((runtime) => {
      if (filter !== "all" && runtime.status !== filter) return false;
      if (!needle) return true;
      return (
        runtime.config.name.toLowerCase().includes(needle) ||
        String(runtime.config.port ?? "").includes(needle)
      );
    });
  }, [runtimes, query, filter]);

  const groups = useMemo(() => {
    const byGroup = new Map<string, ServiceRuntime[]>();
    for (const runtime of visible) {
      const key = runtime.config.group?.trim() || UNGROUPED;
      const bucket = byGroup.get(key);
      if (bucket) bucket.push(runtime);
      else byGroup.set(key, [runtime]);
    }
    return [...byGroup.entries()];
  }, [visible]);

  const toggleGroup = (key: string) => {
    setCollapsed((current) => {
      const next = new Set(current);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  return (
    <div className="flex min-h-0 flex-col">
      <div className="flex flex-col gap-2 border-b border-(--color-border) p-2">
        <div className="flex gap-1.5">
          <div className="relative flex-1">
            <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-(--color-muted-soft)" />
            <Input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Filtrer"
              className="h-8 pl-8 text-xs"
            />
          </div>
          <button
            type="button"
            onClick={onAdd}
            title="Ajouter un service"
            aria-label="Ajouter un service"
            className="shrink-0 rounded-md border border-(--color-border) px-2 text-(--color-muted) transition-colors hover:bg-(--color-panel-hover) hover:text-(--color-text)"
          >
            <Plus className="h-3.5 w-3.5" />
          </button>
        </div>
        <div className="flex flex-wrap gap-1">
          {FILTERS.map((entry) => (
            <button
              key={entry.id}
              type="button"
              onClick={() => setFilter(entry.id)}
              className={cn(
                "rounded px-2 py-0.5 text-[11px] transition-colors",
                filter === entry.id
                  ? "bg-(--color-accent-bg) text-(--color-accent)"
                  : "text-(--color-muted) hover:bg-(--color-panel-hover) hover:text-(--color-text-soft)",
              )}
            >
              {entry.label}
            </button>
          ))}
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto p-1">
        {visible.length === 0 && (
          <p className="px-3 py-6 text-center text-xs text-(--color-muted)">Aucun service</p>
        )}

        {groups.map(([key, entries]) => {
          const ids = entries.map((entry) => entry.id);
          const allChecked = ids.every((id) => checked.has(id));
          const isCollapsed = collapsed.has(key);
          return (
            <section key={key}>
              {key !== UNGROUPED && (
                <div className="flex items-center gap-1.5 px-1.5 pb-0.5 pt-2">
                  <button
                    type="button"
                    onClick={() => toggleGroup(key)}
                    className="flex flex-1 items-center gap-1 text-[10px] font-semibold uppercase tracking-wider text-(--color-muted) hover:text-(--color-text-soft)"
                  >
                    {isCollapsed ? (
                      <ChevronRight className="h-3 w-3" />
                    ) : (
                      <ChevronDown className="h-3 w-3" />
                    )}
                    <span className="truncate">{key}</span>
                    <span className="text-(--color-muted-soft)">{entries.length}</span>
                  </button>
                  <Checkbox
                    checked={allChecked}
                    onCheckedChange={(value) => onCheckMany(ids, value)}
                  />
                </div>
              )}

              {!isCollapsed &&
                entries.map((runtime) => (
                  <ServiceRow
                    key={runtime.id}
                    runtime={runtime}
                    active={runtime.id === selectedId}
                    checked={checked.has(runtime.id)}
                    onCheck={(value) => onCheck(runtime.id, value)}
                    onSelect={() => onSelect(runtime.id)}
                  />
                ))}
            </section>
          );
        })}
      </div>
    </div>
  );
}

type RowProps = {
  runtime: ServiceRuntime;
  active: boolean;
  checked: boolean;
  onCheck: (value: boolean) => void;
  onSelect: () => void;
};

function ServiceRow({ runtime, active, checked, onCheck, onSelect }: RowProps) {
  const tone = statusTone(runtime.status);
  const uptime =
    runtime.status === "running" && runtime.startedAt ? formatUptime(runtime.startedAt) : "";

  return (
    <div
      className={cn(
        "group flex items-center gap-2 rounded-md px-1.5 py-1.5 transition-colors",
        active ? "bg-(--color-accent-bg)" : "hover:bg-(--color-panel-hover)",
      )}
    >
      <Checkbox checked={checked} onCheckedChange={onCheck} />

      <button
        type="button"
        onClick={onSelect}
        className="flex min-w-0 flex-1 items-center gap-2 text-left"
      >
        <StatusDot status={runtime.status} />
        <span
          className={cn(
            "min-w-0 flex-1 truncate text-xs",
            active ? "font-medium text-(--color-text)" : "text-(--color-text-soft)",
          )}
        >
          {runtime.config.name}
        </span>

        {runtime.stuck && (
          <span className="rounded bg-(--color-panel) px-1 text-[9px] font-medium uppercase tracking-wide text-(--color-danger)">
            bloqué
          </span>
        )}
        {!runtime.stuck && (runtime.retryCount ?? 0) > 0 && (
          <span className="font-mono text-[10px] text-(--color-warning)">
            ↻{runtime.retryCount}
          </span>
        )}

        {runtime.config.port && (
          <span className="font-mono text-[10px] text-(--color-muted-soft)">
            :{runtime.config.port}
          </span>
        )}
        {uptime && <span className="font-mono text-[10px] text-(--color-muted)">{uptime}</span>}
        {!uptime && runtime.status !== "running" && (
          <span className={cn("text-[10px]", TONE_TEXT[tone])}>
            {runtime.status === "stopped"
              ? TYPE_LABEL[runtime.config.type]
              : STATUS_LABEL[runtime.status]}
          </span>
        )}
      </button>
    </div>
  );
}
