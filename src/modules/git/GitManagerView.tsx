import { GitBranch } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { Button } from "@/components/ui/Button";
import { formatError, toastError, toastInfo, toastSuccess } from "@/lib/feedback";
import type { CheckOptions } from "@/lib/selection";
import { rangeBetween } from "@/lib/selection";

import { GitProfileBar } from "./components/GitProfileBar";
import { NewGitProfileDialog } from "./components/NewGitProfileDialog";
import { RepoDetail } from "./components/RepoDetail";
import { RepoList } from "./components/RepoList";
import { useGitStore } from "./store";
import type { BatchOutcome } from "./types";

/**
 * Git Manager — écran principal.
 *
 * Trois zones : la barre de pilotage (profil, compteurs, opérations réseau
 * groupées), la liste des dépôts, et le détail du dépôt courant.
 */
export function GitManagerView() {
  const hydrate = useGitStore((s) => s.hydrate);
  const profiles = useGitStore((s) => s.profiles);
  const activeProfileId = useGitStore((s) => s.activeProfileId);
  const repoMap = useGitStore((s) => s.repos);
  const selectedRepoPath = useGitStore((s) => s.selectedRepoPath);
  const busy = useGitStore((s) => s.busy);
  const selectProfile = useGitStore((s) => s.selectProfile);
  const deleteProfile = useGitStore((s) => s.deleteProfile);
  const selectRepo = useGitStore((s) => s.selectRepo);
  const refreshAll = useGitStore((s) => s.refreshAll);
  const fetchMany = useGitStore((s) => s.fetchMany);
  const pullMany = useGitStore((s) => s.pullMany);
  const pushMany = useGitStore((s) => s.pushMany);

  const [checked, setChecked] = useState<Set<string>>(new Set());
  /** Dernière ligne cochée à la main : point de départ d'un Maj+clic. */
  const [anchor, setAnchor] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  const repos = useMemo(() => Object.values(repoMap), [repoMap]);
  const selected = selectedRepoPath ? repoMap[selectedRepoPath] : undefined;
  const selection = useMemo(
    () => repos.filter((repo) => checked.has(repo.path)).map((repo) => repo.path),
    [repos, checked],
  );

  const check = (repoPath: string, value: boolean, options: CheckOptions) => {
    const targets =
      options.shiftKey && anchor ? rangeBetween(options.ordered, anchor, repoPath) : [repoPath];
    setChecked((current) => {
      const next = new Set(current);
      for (const path of targets) {
        if (value) next.add(path);
        else next.delete(path);
      }
      return next;
    });
    // L'ancre ne bouge pas pendant un Maj+clic : on peut étendre la plage
    // plusieurs fois depuis le même point de départ.
    if (!options.shiftKey) setAnchor(repoPath);
  };

  const checkAll = (value: boolean) => {
    setChecked(value ? new Set(repos.map((repo) => repo.path)) : new Set());
  };

  /**
   * Compte rendu d'une opération groupée.
   *
   * On nomme les dépôts en échec : sur douze dépôts, « 2 échecs » sans dire
   * lesquels oblige à tout rouvrir un par un.
   */
  const reportOutcomes = (label: string, outcomes: BatchOutcome[]) => {
    const failed = outcomes.filter((outcome) => !outcome.success);
    if (failed.length === 0) {
      toastSuccess(`${label} — ${outcomes.length} dépôt${outcomes.length > 1 ? "s" : ""} à jour`);
      return;
    }
    const names = failed.map((outcome) => outcome.repoName).join(", ");
    if (failed.length === outcomes.length) {
      toastError(`${label} — échec sur ${names}`);
    } else {
      toastInfo(`${label} — ${outcomes.length - failed.length} ok, échec sur ${names}`);
    }
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <GitProfileBar
        profiles={profiles}
        activeProfileId={activeProfileId}
        repos={repos}
        selection={selection}
        busy={busy}
        onSelectProfile={(profileId) => {
          setChecked(new Set());
          void selectProfile(profileId).catch((error) => toastError(formatError(error)));
        }}
        onNewProfile={() => setCreating(true)}
        onDeleteProfile={() => {
          if (!activeProfileId) return;
          setChecked(new Set());
          void deleteProfile(activeProfileId).catch((error) => toastError(formatError(error)));
        }}
        onClearSelection={() => {
          setChecked(new Set());
          setAnchor(null);
        }}
        onRefresh={() => void refreshAll()}
        onFetch={fetchMany}
        onPull={pullMany}
        onPush={pushMany}
        onOutcomes={reportOutcomes}
      />

      {profiles.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-3">
          <GitBranch className="h-8 w-8 text-(--color-muted-soft)" />
          <p className="text-sm text-(--color-muted)">Aucun profil</p>
          <Button size="sm" onClick={() => setCreating(true)}>
            Analyser un dossier
          </Button>
        </div>
      ) : (
        <div className="flex min-h-0 flex-1">
          <aside className="flex w-72 shrink-0 flex-col border-r border-(--color-border) bg-(--color-bg-soft)">
            <RepoList
              repos={repos}
              selectedPath={selectedRepoPath}
              onSelect={selectRepo}
              checked={checked}
              onCheck={check}
              onCheckAll={checkAll}
            />
          </aside>

          {selected ? (
            <RepoDetail key={selected.path} repo={selected} />
          ) : (
            <div className="flex flex-1 items-center justify-center">
              <p className="text-xs text-(--color-muted)">Choisissez un dépôt</p>
            </div>
          )}
        </div>
      )}

      <NewGitProfileDialog open={creating} onOpenChange={setCreating} />
    </div>
  );
}
