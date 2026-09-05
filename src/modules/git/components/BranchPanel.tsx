import { Check, GitMerge, Plus, Trash2 } from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { formatError, toastError } from "@/lib/feedback";
import { cn } from "@/lib/utils";

import { useGitStore } from "../store";
import type { RepoState } from "../types";

type Props = { repo: RepoState };

export function BranchPanel({ repo }: Props) {
  const checkout = useGitStore((s) => s.checkout);
  const createBranch = useGitStore((s) => s.createBranch);
  const deleteBranch = useGitStore((s) => s.deleteBranch);
  const merge = useGitStore((s) => s.merge);

  const [newBranch, setNewBranch] = useState("");

  const guard = (action: Promise<void>) => {
    void action.catch((error) => toastError(formatError(error)));
  };

  const current = repo.branches?.current ?? "";
  const local = repo.branches?.local ?? [];
  const remote = repo.branches?.remote ?? [];

  return (
    <div className="min-h-0 flex-1 overflow-y-auto p-3">
      <div className="mb-4 flex gap-2">
        <Input
          value={newBranch}
          onChange={(event) => setNewBranch(event.target.value)}
          onKeyDown={(event) => {
            if (event.key !== "Enter" || !newBranch.trim()) return;
            guard(createBranch(repo.path, newBranch.trim()));
            setNewBranch("");
          }}
          placeholder="Nouvelle branche"
          className="h-8 text-xs"
        />
        <Button
          size="sm"
          variant="outline"
          disabled={!newBranch.trim()}
          onClick={() => {
            guard(createBranch(repo.path, newBranch.trim()));
            setNewBranch("");
          }}
        >
          <Plus className="h-3.5 w-3.5" />
          Créer
        </Button>
      </div>

      <Group title="Locales" count={local.length}>
        {local.map((branch) => {
          const isCurrent = branch === current;
          return (
            <div
              key={branch}
              className={cn(
                "group flex items-center gap-2 rounded px-1.5 py-1 transition-colors",
                isCurrent ? "bg-(--color-accent-bg)" : "hover:bg-(--color-panel-hover)",
              )}
            >
              <span className="w-3">
                {isCurrent && <Check className="h-3 w-3 text-(--color-accent)" />}
              </span>
              <button
                type="button"
                disabled={isCurrent}
                onClick={() => guard(checkout(repo.path, branch))}
                className={cn(
                  "min-w-0 flex-1 truncate text-left font-mono text-xs",
                  isCurrent
                    ? "cursor-default text-(--color-text)"
                    : "text-(--color-text-soft) hover:text-(--color-text)",
                )}
              >
                {branch}
              </button>

              {!isCurrent && (
                <span className="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
                  <IconAction
                    title={`Fusionner ${branch} dans ${current}`}
                    onClick={() => guard(merge(repo.path, branch))}
                  >
                    <GitMerge className="h-3 w-3" />
                  </IconAction>
                  <IconAction
                    title="Supprimer la branche"
                    danger
                    onClick={() => guard(deleteBranch(repo.path, branch, false))}
                  >
                    <Trash2 className="h-3 w-3" />
                  </IconAction>
                </span>
              )}
            </div>
          );
        })}
      </Group>

      <Group title="Distantes" count={remote.length}>
        {remote.map((branch) => (
          <div
            key={branch}
            className="group flex items-center gap-2 rounded px-1.5 py-1 transition-colors hover:bg-(--color-panel-hover)"
          >
            <span className="w-3" />
            <button
              type="button"
              // `git checkout origin/x` crée la branche locale de suivi si elle
              // n'existe pas encore — c'est le geste attendu.
              onClick={() => guard(checkout(repo.path, branch.replace(/^[^/]+\//, "")))}
              className="min-w-0 flex-1 truncate text-left font-mono text-xs text-(--color-muted) hover:text-(--color-text-soft)"
            >
              {branch}
            </button>
          </div>
        ))}
        {remote.length === 0 && (
          <p className="px-1.5 py-2 text-xs text-(--color-muted-soft)">Aucune branche distante</p>
        )}
      </Group>
    </div>
  );
}

function Group({
  title,
  count,
  children,
}: {
  title: string;
  count: number;
  children: React.ReactNode;
}) {
  return (
    <section className="pb-3">
      <div className="flex items-center gap-2 px-1.5 pb-1">
        <span className="text-[10px] font-semibold uppercase tracking-wider text-(--color-muted)">
          {title}
        </span>
        <span className="text-[10px] text-(--color-muted-soft)">{count}</span>
      </div>
      {children}
    </section>
  );
}

function IconAction({
  title,
  danger,
  onClick,
  children,
}: {
  title: string;
  danger?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      title={title}
      aria-label={title}
      onClick={onClick}
      className={cn(
        "rounded p-0.5 text-(--color-muted) transition-colors",
        danger ? "hover:text-(--color-danger)" : "hover:text-(--color-accent)",
      )}
    >
      {children}
    </button>
  );
}
