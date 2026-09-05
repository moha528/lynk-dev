import { ArrowDownToLine, Trash2 } from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { formatError, toastError } from "@/lib/feedback";

import { useGitStore } from "../store";
import type { RepoState } from "../types";

type Props = { repo: RepoState };

export function StashPanel({ repo }: Props) {
  const stashSave = useGitStore((s) => s.stashSave);
  const stashPop = useGitStore((s) => s.stashPop);
  const stashDrop = useGitStore((s) => s.stashDrop);

  const [message, setMessage] = useState("");

  const guard = (action: Promise<void>) => {
    void action.catch((error) => toastError(formatError(error)));
  };

  const save = () => {
    guard(stashSave(repo.path, message.trim() || undefined));
    setMessage("");
  };

  return (
    <div className="min-h-0 flex-1 overflow-y-auto p-3">
      <div className="mb-4 flex gap-2">
        <Input
          value={message}
          onChange={(event) => setMessage(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") save();
          }}
          placeholder="Étiquette (facultative)"
          className="h-8 text-xs"
        />
        <Button size="sm" variant="outline" onClick={save}>
          Remiser
        </Button>
      </div>

      {repo.stashes.length === 0 ? (
        <p className="py-6 text-center text-xs text-(--color-muted)">Aucun remisage</p>
      ) : (
        repo.stashes.map((stash) => (
          <div
            key={stash.index}
            className="group flex items-center gap-2 rounded px-1.5 py-1.5 hover:bg-(--color-panel-hover)"
          >
            <span className="shrink-0 font-mono text-[10px] text-(--color-muted-soft)">
              {stash.index}
            </span>
            <div className="min-w-0 flex-1">
              <p className="truncate text-xs text-(--color-text-soft)" title={stash.message}>
                {stash.message}
              </p>
              <p className="truncate text-[10px] text-(--color-muted-soft)">{stash.date}</p>
            </div>
            <span className="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100">
              <button
                type="button"
                title="Appliquer et retirer"
                aria-label="Appliquer et retirer"
                onClick={() => guard(stashPop(repo.path, stash.index))}
                className="rounded p-0.5 text-(--color-muted) transition-colors hover:text-(--color-accent)"
              >
                <ArrowDownToLine className="h-3.5 w-3.5" />
              </button>
              <button
                type="button"
                title="Supprimer le remisage"
                aria-label="Supprimer le remisage"
                onClick={() => guard(stashDrop(repo.path, stash.index))}
                className="rounded p-0.5 text-(--color-muted) transition-colors hover:text-(--color-danger)"
              >
                <Trash2 className="h-3.5 w-3.5" />
              </button>
            </span>
          </div>
        ))
      )}
    </div>
  );
}
