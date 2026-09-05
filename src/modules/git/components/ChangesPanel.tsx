import { Minus, Plus, Undo2 } from "lucide-react";
import { useMemo, useState } from "react";

import { Button } from "@/components/ui/Button";
import { formatError, toastError } from "@/lib/feedback";
import { cn } from "@/lib/utils";

import { useGitStore } from "../store";
import type { FileChange, FileStatus, RepoState } from "../types";
import { DiffViewer } from "./DiffViewer";

type Props = { repo: RepoState };

type Selection = { path: string; staged: boolean };

const STATUS_MARK: Record<FileStatus, { label: string; className: string }> = {
  added: { label: "A", className: "text-(--color-success)" },
  modified: { label: "M", className: "text-(--color-warning)" },
  deleted: { label: "D", className: "text-(--color-danger)" },
  renamed: { label: "R", className: "text-(--color-accent)" },
  copied: { label: "C", className: "text-(--color-accent)" },
  untracked: { label: "?", className: "text-(--color-muted)" },
};

export function ChangesPanel({ repo }: Props) {
  const stage = useGitStore((s) => s.stage);
  const unstage = useGitStore((s) => s.unstage);
  const stageAll = useGitStore((s) => s.stageAll);
  const discard = useGitStore((s) => s.discard);
  const commit = useGitStore((s) => s.commit);
  const resolveConflict = useGitStore((s) => s.resolveConflict);
  const mergeAbort = useGitStore((s) => s.mergeAbort);

  const [selection, setSelection] = useState<Selection | null>(null);
  const [message, setMessage] = useState("");
  const [committing, setCommitting] = useState(false);

  const status = repo.status;
  const untrackedAsChanges = useMemo<FileChange[]>(
    () => (status?.untracked ?? []).map((path) => ({ path, status: "untracked" as const })),
    [status],
  );

  const guard = (action: Promise<void>) => {
    void action.catch((error) => toastError(formatError(error)));
  };

  const doCommit = async () => {
    if (!message.trim()) return;
    setCommitting(true);
    try {
      await commit(repo.path, message.trim());
      setMessage("");
      setSelection(null);
    } catch (error) {
      toastError(formatError(error));
    } finally {
      setCommitting(false);
    }
  };

  if (!status) {
    return <p className="p-4 text-xs text-(--color-muted)">…</p>;
  }

  const nothing =
    status.staged.length === 0 &&
    status.modified.length === 0 &&
    status.untracked.length === 0 &&
    status.conflicts.length === 0;

  return (
    <div className="flex min-h-0 flex-1">
      <div className="flex w-80 shrink-0 flex-col border-r border-(--color-border)">
        <div className="min-h-0 flex-1 overflow-y-auto p-1">
          {nothing && (
            <p className="px-3 py-6 text-center text-xs text-(--color-muted)">Rien à valider</p>
          )}

          {status.conflicts.length > 0 && (
            <Section
              title="Conflits"
              count={status.conflicts.length}
              tone="text-(--color-danger)"
              action={
                <button
                  type="button"
                  onClick={() => guard(mergeAbort(repo.path))}
                  className="text-[10px] text-(--color-muted) hover:text-(--color-danger)"
                >
                  abandonner la fusion
                </button>
              }
            >
              {status.conflicts.map((conflict) => (
                <div
                  key={conflict.path}
                  className="flex items-center gap-2 rounded px-1.5 py-1 hover:bg-(--color-panel-hover)"
                >
                  <span className="font-mono text-[10px] text-(--color-danger)">
                    {conflict.oursStatus}
                    {conflict.theirsStatus}
                  </span>
                  <span className="min-w-0 flex-1 truncate text-xs text-(--color-text-soft)">
                    {conflict.path}
                  </span>
                  <button
                    type="button"
                    title="Garder notre version"
                    onClick={() => guard(resolveConflict(repo.path, conflict.path, "ours"))}
                    className="rounded px-1 text-[10px] text-(--color-muted) hover:text-(--color-accent)"
                  >
                    nôtre
                  </button>
                  <button
                    type="button"
                    title="Garder leur version"
                    onClick={() => guard(resolveConflict(repo.path, conflict.path, "theirs"))}
                    className="rounded px-1 text-[10px] text-(--color-muted) hover:text-(--color-accent)"
                  >
                    leur
                  </button>
                </div>
              ))}
            </Section>
          )}

          {status.staged.length > 0 && (
            <Section
              title="Indexé"
              count={status.staged.length}
              action={
                <button
                  type="button"
                  onClick={() =>
                    guard(
                      unstage(
                        repo.path,
                        status.staged.map((file) => file.path),
                      ),
                    )
                  }
                  className="text-[10px] text-(--color-muted) hover:text-(--color-text)"
                >
                  tout retirer
                </button>
              }
            >
              {status.staged.map((file) => (
                <FileRow
                  key={file.path}
                  file={file}
                  active={selection?.path === file.path && selection.staged}
                  onSelect={() => setSelection({ path: file.path, staged: true })}
                  actions={
                    <RowButton
                      title="Retirer de l'index"
                      onClick={() => guard(unstage(repo.path, [file.path]))}
                    >
                      <Minus className="h-3 w-3" />
                    </RowButton>
                  }
                />
              ))}
            </Section>
          )}

          {(status.modified.length > 0 || untrackedAsChanges.length > 0) && (
            <Section
              title="Modifié"
              count={status.modified.length + untrackedAsChanges.length}
              action={
                <button
                  type="button"
                  onClick={() => guard(stageAll(repo.path))}
                  className="text-[10px] text-(--color-muted) hover:text-(--color-text)"
                >
                  tout indexer
                </button>
              }
            >
              {[...status.modified, ...untrackedAsChanges].map((file) => (
                <FileRow
                  key={`${file.status}-${file.path}`}
                  file={file}
                  active={selection?.path === file.path && !selection.staged}
                  onSelect={() => setSelection({ path: file.path, staged: false })}
                  actions={
                    <>
                      <RowButton
                        title="Abandonner les modifications"
                        danger
                        onClick={() =>
                          guard(discard(repo.path, [file.path], file.status === "untracked"))
                        }
                      >
                        <Undo2 className="h-3 w-3" />
                      </RowButton>
                      <RowButton
                        title="Indexer"
                        onClick={() => guard(stage(repo.path, [file.path]))}
                      >
                        <Plus className="h-3 w-3" />
                      </RowButton>
                    </>
                  }
                />
              ))}
            </Section>
          )}
        </div>

        <div className="flex flex-col gap-2 border-t border-(--color-border) p-2">
          <textarea
            value={message}
            onChange={(event) => setMessage(event.target.value)}
            placeholder="Message de validation"
            rows={3}
            className={cn(
              "w-full resize-none rounded-md border border-(--color-border) bg-(--color-bg) px-2 py-1.5 text-xs text-(--color-text)",
              "placeholder:text-(--color-muted-soft)",
              "focus-visible:border-(--color-accent) focus-visible:outline-none",
            )}
          />
          <Button
            size="sm"
            disabled={committing || !message.trim() || status.staged.length === 0}
            onClick={() => void doCommit()}
          >
            Valider {status.staged.length > 0 && `(${status.staged.length})`}
          </Button>
        </div>
      </div>

      <div className="min-h-0 min-w-0 flex-1 overflow-auto bg-(--color-bg)">
        {selection ? (
          <DiffViewer
            key={`${selection.path}-${selection.staged}`}
            repoPath={repo.path}
            filePath={selection.path}
            staged={selection.staged}
          />
        ) : (
          <p className="p-6 text-center text-xs text-(--color-muted)">Choisissez un fichier</p>
        )}
      </div>
    </div>
  );
}

function Section({
  title,
  count,
  tone,
  action,
  children,
}: {
  title: string;
  count: number;
  tone?: string;
  action?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <section className="pb-2">
      <div className="flex items-center gap-2 px-1.5 pb-0.5 pt-2">
        <span
          className={cn(
            "text-[10px] font-semibold uppercase tracking-wider",
            tone ?? "text-(--color-muted)",
          )}
        >
          {title}
        </span>
        <span className="text-[10px] text-(--color-muted-soft)">{count}</span>
        <span className="ml-auto">{action}</span>
      </div>
      {children}
    </section>
  );
}

function FileRow({
  file,
  active,
  onSelect,
  actions,
}: {
  file: FileChange;
  active: boolean;
  onSelect: () => void;
  actions: React.ReactNode;
}) {
  const mark = STATUS_MARK[file.status];
  return (
    <div
      className={cn(
        "group flex items-center gap-2 rounded px-1.5 py-1 transition-colors",
        active ? "bg-(--color-accent-bg)" : "hover:bg-(--color-panel-hover)",
      )}
    >
      <button type="button" onClick={onSelect} className="flex min-w-0 flex-1 items-center gap-2">
        <span className={cn("w-3 font-mono text-[10px]", mark.className)}>{mark.label}</span>
        <span
          className={cn(
            "min-w-0 flex-1 truncate text-left text-xs",
            active ? "text-(--color-text)" : "text-(--color-text-soft)",
          )}
          title={file.oldPath ? `${file.oldPath} → ${file.path}` : file.path}
        >
          {file.path}
        </span>
      </button>
      <span className="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
        {actions}
      </span>
    </div>
  );
}

function RowButton({
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
