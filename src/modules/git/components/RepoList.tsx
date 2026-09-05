import { ArrowDown, ArrowUp, Search } from "lucide-react";
import { useMemo, useRef, useState } from "react";

import { Checkbox } from "@/components/ui/Checkbox";
import { Input } from "@/components/ui/Input";
import type { CheckOptions } from "@/lib/selection";
import { neighbour } from "@/lib/selection";
import { cn } from "@/lib/utils";

import { dirtyCount } from "../types";
import type { RepoState } from "../types";

type Props = {
  repos: RepoState[];
  selectedPath: string | null;
  onSelect: (repoPath: string) => void;
  checked: Set<string>;
  onCheck: (repoPath: string, value: boolean, options: CheckOptions) => void;
  onCheckAll: (value: boolean) => void;
};

export function RepoList({ repos, selectedPath, onSelect, checked, onCheck, onCheckAll }: Props) {
  const [query, setQuery] = useState("");
  const [onlyDirty, setOnlyDirty] = useState(false);
  // `onCheckedChange` ne transmet pas l'événement : on retient l'état de la
  // touche Maj au tout début du clic, avant que la case ne réagisse.
  const shiftHeld = useRef(false);

  const visible = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return repos.filter((repo) => {
      if (onlyDirty && dirtyCount(repo.status) === 0) return false;
      if (!needle) return true;
      return (
        repo.name.toLowerCase().includes(needle) ||
        (repo.status?.branch ?? "").toLowerCase().includes(needle)
      );
    });
  }, [repos, query, onlyDirty]);

  const allChecked = visible.length > 0 && visible.every((repo) => checked.has(repo.path));
  /** Ordre réel à l'écran, filtres compris. */
  const ordered = useMemo(() => visible.map((repo) => repo.path), [visible]);

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const next = neighbour(ordered, selectedPath, event.key === "ArrowDown" ? 1 : -1);
      if (next) onSelect(next);
      return;
    }
    if (event.key === " " && selectedPath) {
      event.preventDefault();
      onCheck(selectedPath, !checked.has(selectedPath), { shiftKey: event.shiftKey, ordered });
      return;
    }
    if (event.key.toLowerCase() === "a" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      onCheckAll(true);
    }
  };

  return (
    <div className="flex min-h-0 flex-col">
      <div className="flex flex-col gap-2 border-b border-(--color-border) p-2">
        <div className="relative">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-(--color-muted-soft)" />
          <Input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Filtrer"
            className="h-8 pl-8 text-xs"
          />
        </div>
        <div className="flex items-center gap-2">
          <Checkbox checked={allChecked} onCheckedChange={onCheckAll} />
          <button
            type="button"
            onClick={() => setOnlyDirty((value) => !value)}
            className={cn(
              "rounded px-2 py-0.5 text-[11px] transition-colors",
              onlyDirty
                ? "bg-(--color-accent-bg) text-(--color-accent)"
                : "text-(--color-muted) hover:bg-(--color-panel-hover) hover:text-(--color-text-soft)",
            )}
          >
            Modifiés
          </button>
          <span className="ml-auto text-[10px] text-(--color-muted-soft)">
            Maj+clic pour une plage
          </span>
        </div>
      </div>

      {/* biome-ignore lint/a11y/useSemanticElements: une liste de lignes à la
          fois sélectionnables et cochables n'a pas d'équivalent natif. */}
      <div
        role="listbox"
        aria-label="Dépôts"
        tabIndex={0}
        onKeyDown={onKeyDown}
        onMouseDownCapture={(event) => {
          shiftHeld.current = event.shiftKey;
        }}
        className="min-h-0 flex-1 overflow-y-auto p-1 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-(--color-accent)"
      >
        {visible.length === 0 && (
          <p className="px-3 py-6 text-center text-xs text-(--color-muted)">Aucun dépôt</p>
        )}
        {visible.map((repo) => (
          <RepoRow
            key={repo.path}
            repo={repo}
            active={repo.path === selectedPath}
            checked={checked.has(repo.path)}
            onCheck={(value) => onCheck(repo.path, value, { shiftKey: shiftHeld.current, ordered })}
            onSelect={() => onSelect(repo.path)}
          />
        ))}
      </div>
    </div>
  );
}

function RepoRow({
  repo,
  active,
  checked,
  onCheck,
  onSelect,
}: {
  repo: RepoState;
  active: boolean;
  checked: boolean;
  onCheck: (value: boolean) => void;
  onSelect: () => void;
}) {
  const dirty = dirtyCount(repo.status);
  const conflicts = repo.status?.conflicts.length ?? 0;

  return (
    <div
      className={cn(
        "flex items-center gap-2 rounded-md px-1.5 py-1.5 transition-colors",
        active ? "bg-(--color-accent-bg)" : "hover:bg-(--color-panel-hover)",
        checked && !active && "bg-(--color-panel)",
      )}
    >
      <Checkbox checked={checked} onCheckedChange={onCheck} />

      <button type="button" onClick={onSelect} className="min-w-0 flex-1 text-left">
        <div className="flex items-center gap-2">
          <span
            className={cn(
              "min-w-0 flex-1 truncate text-xs",
              active ? "font-medium text-(--color-text)" : "text-(--color-text-soft)",
            )}
          >
            {repo.name}
          </span>

          {conflicts > 0 && (
            <span className="shrink-0 rounded bg-(--color-panel) px-1 text-[9px] font-medium uppercase text-(--color-danger)">
              conflit
            </span>
          )}
          {conflicts === 0 && dirty > 0 && (
            <span className="shrink-0 font-mono text-[10px] text-(--color-warning)">{dirty}</span>
          )}
        </div>

        <div className="flex items-center gap-2 pt-0.5">
          <span className="min-w-0 flex-1 truncate font-mono text-[10px] text-(--color-muted)">
            {repo.error ? "—" : (repo.status?.branch ?? "…")}
          </span>
          {(repo.status?.ahead ?? 0) > 0 && (
            <span className="flex shrink-0 items-center font-mono text-[10px] text-(--color-success)">
              <ArrowUp className="h-2.5 w-2.5" />
              {repo.status?.ahead}
            </span>
          )}
          {(repo.status?.behind ?? 0) > 0 && (
            <span className="flex shrink-0 items-center font-mono text-[10px] text-(--color-accent)">
              <ArrowDown className="h-2.5 w-2.5" />
              {repo.status?.behind}
            </span>
          )}
        </div>
      </button>
    </div>
  );
}
