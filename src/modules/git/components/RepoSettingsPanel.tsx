import { Trash2 } from "lucide-react";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { formatError, toastError, toastSuccess } from "@/lib/feedback";
import { cn } from "@/lib/utils";

import { gitApi } from "../ipc";
import { useGitStore } from "../store";
import type { RepoState } from "../types";

type Props = { repo: RepoState };

export function RepoSettingsPanel({ repo }: Props) {
  const loadConfig = useGitStore((s) => s.loadConfig);
  const [remoteName, setRemoteName] = useState("");
  const [remoteUrl, setRemoteUrl] = useState("");
  const [identity, setIdentity] = useState({ name: "", email: "" });

  const config = repo.config;

  useEffect(() => {
    void loadConfig(repo.path);
  }, [loadConfig, repo.path]);

  // La configuration arrive après coup : on ne pré-remplit qu'une fois, sinon
  // chaque rafraîchissement écraserait ce que l'utilisateur est en train de
  // taper.
  useEffect(() => {
    if (!config) return;
    setIdentity({ name: config.userName ?? "", email: config.userEmail ?? "" });
  }, [config]);

  const reload = () => void loadConfig(repo.path);

  const run = (action: Promise<unknown>, success: string) => {
    action
      .then(() => {
        toastSuccess(success);
        reload();
      })
      .catch((error: unknown) => toastError(formatError(error)));
  };

  if (!config) {
    return <p className="p-4 text-xs text-(--color-muted)">…</p>;
  }

  return (
    <div className="min-h-0 flex-1 overflow-y-auto p-3">
      <Section title="Dépôts distants">
        {config.remotes.length === 0 && (
          <p className="pb-2 text-xs text-(--color-muted-soft)">Aucun dépôt distant</p>
        )}
        {config.remotes.map((remote) => (
          <div key={remote.name} className="group flex items-center gap-2 py-1">
            <span className="w-16 shrink-0 truncate font-mono text-[11px] text-(--color-accent)">
              {remote.name}
            </span>
            <span
              className="min-w-0 flex-1 truncate font-mono text-[11px] text-(--color-text-soft)"
              title={remote.fetchUrl}
            >
              {remote.fetchUrl}
            </span>
            {remote.pushUrl !== remote.fetchUrl && (
              <span
                className="shrink-0 rounded bg-(--color-panel) px-1 text-[9px] text-(--color-muted)"
                title={remote.pushUrl}
              >
                push ≠ fetch
              </span>
            )}
            <button
              type="button"
              title="Retirer ce distant"
              aria-label="Retirer ce distant"
              onClick={() =>
                run(gitApi.removeRemote(repo.path, remote.name), `${remote.name} retiré`)
              }
              className="shrink-0 rounded p-0.5 text-(--color-muted) opacity-0 transition-opacity hover:text-(--color-danger) group-hover:opacity-100"
            >
              <Trash2 className="h-3.5 w-3.5" />
            </button>
          </div>
        ))}

        <div className="mt-2 flex gap-2">
          <Input
            value={remoteName}
            onChange={(event) => setRemoteName(event.target.value)}
            placeholder="nom"
            className="h-8 w-24 text-xs"
          />
          <Input
            value={remoteUrl}
            onChange={(event) => setRemoteUrl(event.target.value)}
            placeholder="URL"
            className="h-8 flex-1 font-mono text-xs"
          />
          <Button
            size="sm"
            variant="outline"
            disabled={!remoteName.trim() || !remoteUrl.trim()}
            onClick={() => {
              run(
                gitApi.addRemote(repo.path, remoteName.trim(), remoteUrl.trim()),
                `${remoteName.trim()} ajouté`,
              );
              setRemoteName("");
              setRemoteUrl("");
            }}
          >
            Ajouter
          </Button>
        </div>
      </Section>

      <Section title="Identité (ce dépôt)">
        <div className="flex gap-2">
          <Input
            value={identity.name}
            onChange={(event) => setIdentity((v) => ({ ...v, name: event.target.value }))}
            placeholder={config.globalUserName ?? "nom"}
            className="h-8 flex-1 text-xs"
          />
          <Input
            value={identity.email}
            onChange={(event) => setIdentity((v) => ({ ...v, email: event.target.value }))}
            placeholder={config.globalUserEmail ?? "courriel"}
            className="h-8 flex-1 font-mono text-xs"
          />
          <Button
            size="sm"
            variant="outline"
            onClick={() => {
              const tasks = [
                gitApi.setConfig(repo.path, "user.name", identity.name, false),
                gitApi.setConfig(repo.path, "user.email", identity.email, false),
              ];
              run(Promise.all(tasks), "Identité enregistrée");
            }}
          >
            Enregistrer
          </Button>
        </div>
        {(config.userName === null || config.userEmail === null) && (
          <p className="pt-1.5 text-[10px] text-(--color-muted-soft)">
            Valeurs globales utilisées : {config.globalUserName ?? "—"} ·{" "}
            {config.globalUserEmail ?? "—"}
          </p>
        )}
      </Section>

      <Section title="Suivi des branches">
        {config.branches.map((branch) => (
          <div key={branch.local} className="flex items-center gap-2 py-0.5">
            <span className="min-w-0 flex-1 truncate font-mono text-[11px] text-(--color-text-soft)">
              {branch.local}
            </span>
            <span
              className={cn(
                "shrink-0 font-mono text-[10px]",
                branch.gone
                  ? "text-(--color-danger)"
                  : branch.remote
                    ? "text-(--color-muted)"
                    : "text-(--color-muted-soft)",
              )}
            >
              {branch.gone ? `${branch.remote} (disparue)` : (branch.remote ?? "aucun suivi")}
            </span>
            {branch.remote && (
              <button
                type="button"
                onClick={() =>
                  run(
                    gitApi.unsetBranchUpstream(repo.path, branch.local),
                    `Suivi retiré pour ${branch.local}`,
                  )
                }
                className="shrink-0 text-[10px] text-(--color-muted-soft) hover:text-(--color-danger)"
              >
                détacher
              </button>
            )}
          </div>
        ))}
      </Section>

      <Section title="Emplacement">
        <dl className="grid grid-cols-[6rem_1fr] gap-x-3 gap-y-1 font-mono text-[11px]">
          <dt className="text-(--color-muted)">worktree</dt>
          <dd className="break-all text-(--color-text-soft)">{config.worktree}</dd>
          <dt className="text-(--color-muted)">git dir</dt>
          <dd className="break-all text-(--color-text-soft)">{config.gitDir}</dd>
        </dl>
      </Section>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="pb-5">
      <h3 className="pb-1.5 text-[10px] font-semibold uppercase tracking-wider text-(--color-muted)">
        {title}
      </h3>
      {children}
    </section>
  );
}
