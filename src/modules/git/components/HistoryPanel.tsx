import { cn } from "@/lib/utils";

import type { LogEntry, RepoState } from "../types";

type Props = { repo: RepoState };

export function HistoryPanel({ repo }: Props) {
  if (repo.log.length === 0) {
    return <p className="p-6 text-center text-xs text-(--color-muted)">Aucun commit</p>;
  }

  return (
    <div className="min-h-0 flex-1 overflow-y-auto p-1">
      {repo.log.map((entry) => (
        <CommitRow key={entry.hash} entry={entry} />
      ))}
    </div>
  );
}

function CommitRow({ entry }: { entry: LogEntry }) {
  const refs = entry.refs
    .split(",")
    .map((ref) => ref.trim())
    .filter(Boolean);

  return (
    <div className="flex items-baseline gap-2 rounded px-2 py-1.5 hover:bg-(--color-panel-hover)">
      <span className="shrink-0 font-mono text-[10px] text-(--color-muted-soft)">
        {entry.shortHash}
      </span>
      <div className="min-w-0 flex-1">
        <p className="truncate text-xs text-(--color-text-soft)" title={entry.message}>
          {entry.message}
        </p>
        <p className="truncate text-[10px] text-(--color-muted-soft)">
          {entry.author} · {formatDate(entry.date)}
        </p>
      </div>
      {refs.length > 0 && (
        <span className="flex shrink-0 flex-wrap justify-end gap-1">
          {refs.map((ref) => (
            <span
              key={ref}
              className={cn(
                "rounded px-1 py-0.5 text-[9px]",
                ref.startsWith("HEAD")
                  ? "bg-(--color-accent-bg) text-(--color-accent)"
                  : "bg-(--color-panel) text-(--color-muted)",
              )}
            >
              {ref.replace("HEAD -> ", "")}
            </span>
          ))}
        </span>
      )}
    </div>
  );
}

/**
 * `%ci` rend `2026-09-05 10:00:00 +0000`. On garde le jour et l'heure, sans le
 * décalage : c'est ce qu'on lit dans un historique.
 */
function formatDate(raw: string): string {
  const [day, time] = raw.split(" ");
  if (!day) return raw;
  return time ? `${day} ${time.slice(0, 5)}` : day;
}
