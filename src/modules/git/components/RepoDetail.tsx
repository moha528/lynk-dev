import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { ArrowDown, ArrowUp, FolderOpen, RefreshCw, Terminal } from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/Button";
import { formatError, toastError } from "@/lib/feedback";
import { cn } from "@/lib/utils";

import { gitApi } from "../ipc";
import { useGitStore } from "../store";
import type { RepoState } from "../types";
import { BranchPanel } from "./BranchPanel";
import { ChangesPanel } from "./ChangesPanel";
import { HistoryPanel } from "./HistoryPanel";
import { RepoSettingsPanel } from "./RepoSettingsPanel";
import { StashPanel } from "./StashPanel";

type Props = { repo: RepoState };

type Tab = "changes" | "branches" | "history" | "stash" | "settings";

const TABS: { id: Tab; label: string }[] = [
  { id: "changes", label: "Modifications" },
  { id: "branches", label: "Branches" },
  { id: "history", label: "Historique" },
  { id: "stash", label: "Remisages" },
  { id: "settings", label: "Réglages" },
];

export function RepoDetail({ repo }: Props) {
  const refreshRepo = useGitStore((s) => s.refreshRepo);
  const fetchMany = useGitStore((s) => s.fetchMany);
  const pullMany = useGitStore((s) => s.pullMany);
  const pushMany = useGitStore((s) => s.pushMany);
  const [tab, setTab] = useState<Tab>("changes");

  const status = repo.status;
  const dirty = repo.status?.conflicts.length ?? 0;

  const single = (run: (paths: string[]) => Promise<{ success: boolean; message: string }[]>) => {
    void run([repo.path])
      .then(([outcome]) => {
        if (outcome && !outcome.success) toastError(outcome.message);
      })
      .catch((error: unknown) => toastError(formatError(error)));
  };

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col">
      <header className="flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-(--color-border) px-3 py-2">
        <h2 className="min-w-0 truncate text-sm font-semibold text-(--color-text)">{repo.name}</h2>
        <span className="font-mono text-[11px] text-(--color-muted)">{status?.branch ?? "…"}</span>
        {(status?.ahead ?? 0) > 0 && (
          <span className="flex items-center font-mono text-[11px] text-(--color-success)">
            <ArrowUp className="h-3 w-3" />
            {status?.ahead}
          </span>
        )}
        {(status?.behind ?? 0) > 0 && (
          <span className="flex items-center font-mono text-[11px] text-(--color-accent)">
            <ArrowDown className="h-3 w-3" />
            {status?.behind}
          </span>
        )}
        {dirty > 0 && (
          <span className="rounded bg-(--color-panel) px-1.5 py-0.5 text-[10px] font-medium uppercase text-(--color-danger)">
            {dirty} conflit{dirty > 1 ? "s" : ""}
          </span>
        )}

        <div className="ml-auto flex items-center gap-1">
          <Button size="sm" variant="outline" onClick={() => single(fetchMany)}>
            Fetch
          </Button>
          <Button size="sm" variant="outline" onClick={() => single(pullMany)}>
            Pull
          </Button>
          <Button size="sm" variant="outline" onClick={() => single(pushMany)}>
            Push
          </Button>
          <IconAction
            label="Rafraîchir"
            onClick={() => void refreshRepo(repo.path)}
            spinning={repo.loading}
          >
            <RefreshCw className="h-3.5 w-3.5" />
          </IconAction>
          <IconAction
            label="Ouvrir un terminal"
            onClick={() =>
              void gitApi
                .openInTerminal(repo.path)
                .catch((error: unknown) => toastError(formatError(error)))
            }
          >
            <Terminal className="h-3.5 w-3.5" />
          </IconAction>
          <IconAction
            label="Ouvrir le dossier"
            onClick={() =>
              void revealItemInDir(repo.path).catch((error: unknown) =>
                toastError(formatError(error)),
              )
            }
          >
            <FolderOpen className="h-3.5 w-3.5" />
          </IconAction>
        </div>
      </header>

      {repo.error && (
        <p className="border-b border-(--color-border) bg-(--color-panel) px-3 py-1.5 font-mono text-[11px] text-(--color-danger)">
          {repo.error}
        </p>
      )}

      <nav className="flex gap-1 border-b border-(--color-border) px-2">
        {TABS.map((entry) => (
          <button
            key={entry.id}
            type="button"
            onClick={() => setTab(entry.id)}
            className={cn(
              "-mb-px border-b-2 px-2 py-1.5 text-xs transition-colors",
              tab === entry.id
                ? "border-(--color-accent) text-(--color-text)"
                : "border-transparent text-(--color-muted) hover:text-(--color-text-soft)",
            )}
          >
            {entry.label}
          </button>
        ))}
      </nav>

      {tab === "changes" && <ChangesPanel repo={repo} />}
      {tab === "branches" && <BranchPanel repo={repo} />}
      {tab === "history" && <HistoryPanel repo={repo} />}
      {tab === "stash" && <StashPanel repo={repo} />}
      {tab === "settings" && <RepoSettingsPanel repo={repo} />}
    </div>
  );
}

function IconAction({
  label,
  spinning,
  onClick,
  children,
}: {
  label: string;
  spinning?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      onClick={onClick}
      className={cn(
        "rounded p-1.5 text-(--color-muted) transition-colors hover:bg-(--color-panel-hover) hover:text-(--color-text)",
        spinning && "animate-spin text-(--color-accent)",
      )}
    >
      {children}
    </button>
  );
}
