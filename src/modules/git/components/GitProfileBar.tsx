import { Plus, RefreshCw, Trash2, X } from "lucide-react";

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/AlertDialog";
import { Button } from "@/components/ui/Button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/Select";
import { cn } from "@/lib/utils";

import { dirtyCount } from "../types";
import type { BatchOutcome, GitProfile, RepoState } from "../types";

type Props = {
  profiles: GitProfile[];
  activeProfileId: string | null;
  repos: RepoState[];
  selection: string[];
  busy: boolean;
  onSelectProfile: (profileId: string) => void;
  onNewProfile: () => void;
  onDeleteProfile: () => void;
  onClearSelection: () => void;
  onRefresh: () => void;
  onFetch: (repoPaths: string[]) => Promise<BatchOutcome[]>;
  onPull: (repoPaths: string[]) => Promise<BatchOutcome[]>;
  onPush: (repoPaths: string[]) => Promise<BatchOutcome[]>;
  onOutcomes: (label: string, outcomes: BatchOutcome[]) => void;
};

/**
 * Barre de pilotage du Git Manager.
 *
 * Même parti pris que le Dev Manager : **un seul jeu de boutons**, dont la
 * portée bascule sur la sélection dès qu'il y en a une.
 */
export function GitProfileBar({
  profiles,
  activeProfileId,
  repos,
  selection,
  busy,
  onSelectProfile,
  onNewProfile,
  onDeleteProfile,
  onClearSelection,
  onRefresh,
  onFetch,
  onPull,
  onPush,
  onOutcomes,
}: Props) {
  const targets = selection.length > 0 ? selection : repos.map((repo) => repo.path);
  const dirty = repos.filter((repo) => dirtyCount(repo.status) > 0).length;
  const conflicts = repos.filter((repo) => (repo.status?.conflicts.length ?? 0) > 0).length;
  const ahead = repos.filter((repo) => (repo.status?.ahead ?? 0) > 0).length;
  const behind = repos.filter((repo) => (repo.status?.behind ?? 0) > 0).length;

  const run = (label: string, action: (repoPaths: string[]) => Promise<BatchOutcome[]>) => {
    void action(targets).then((outcomes) => onOutcomes(label, outcomes));
  };

  return (
    <div className="flex flex-wrap items-center gap-2 border-b border-(--color-border) bg-(--color-panel) px-3 py-2">
      <Select value={activeProfileId ?? ""} onValueChange={onSelectProfile}>
        <SelectTrigger className="w-48" aria-label="Profil">
          <SelectValue placeholder="Aucun profil" />
        </SelectTrigger>
        <SelectContent>
          {profiles.map((profile) => (
            <SelectItem key={profile.id} value={profile.id}>
              {profile.name}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      <button
        type="button"
        onClick={onNewProfile}
        title="Nouveau profil"
        aria-label="Nouveau profil"
        className="rounded-md p-1.5 text-(--color-muted) transition-colors hover:bg-(--color-panel-hover) hover:text-(--color-text)"
      >
        <Plus className="h-4 w-4" />
      </button>

      {activeProfileId && (
        <AlertDialog>
          <AlertDialogTrigger asChild>
            <button
              type="button"
              title="Supprimer le profil"
              aria-label="Supprimer le profil"
              className="rounded-md p-1.5 text-(--color-muted) transition-colors hover:bg-(--color-panel-hover) hover:text-(--color-danger)"
            >
              <Trash2 className="h-4 w-4" />
            </button>
          </AlertDialogTrigger>
          <AlertDialogContent>
            <AlertDialogTitle>Supprimer ce profil ?</AlertDialogTitle>
            <AlertDialogDescription>
              Seule la liste des {repos.length} dépôts est supprimée. Les dépôts eux-mêmes ne sont
              pas touchés.
            </AlertDialogDescription>
            <AlertDialogFooter>
              <AlertDialogCancel asChild>
                <Button variant="outline" size="sm">
                  Annuler
                </Button>
              </AlertDialogCancel>
              <AlertDialogAction asChild>
                <Button variant="destructive" size="sm" onClick={onDeleteProfile}>
                  Supprimer
                </Button>
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      )}

      <div className="flex items-center gap-3 pl-1 text-[11px]">
        {conflicts > 0 && <Counter value={conflicts} label="en conflit" tone="danger" />}
        {dirty > 0 && <Counter value={dirty} label="modifiés" tone="warning" />}
        {ahead > 0 && <Counter value={ahead} label="en avance" tone="success" />}
        {behind > 0 && <Counter value={behind} label="en retard" tone="accent" />}
        <span className="text-(--color-muted-soft)">{repos.length} dépôts</span>
      </div>

      <div className="ml-auto flex items-center gap-2">
        {selection.length > 0 && (
          <button
            type="button"
            onClick={onClearSelection}
            className="inline-flex items-center gap-1 rounded bg-(--color-accent-bg) px-2 py-1 text-[11px] text-(--color-accent) transition-colors hover:brightness-110"
          >
            {selection.length} sélectionné{selection.length > 1 ? "s" : ""}
            <X className="h-3 w-3" />
          </button>
        )}

        <div className="flex items-center gap-1">
          <Button
            size="sm"
            variant="outline"
            disabled={busy || targets.length === 0}
            onClick={() => run("Fetch", onFetch)}
          >
            Fetch
          </Button>
          <Button
            size="sm"
            variant="outline"
            disabled={busy || targets.length === 0}
            onClick={() => run("Pull", onPull)}
          >
            Pull
          </Button>
          <Button
            size="sm"
            variant="outline"
            disabled={busy || targets.length === 0}
            onClick={() => run("Push", onPush)}
          >
            Push
          </Button>
          <button
            type="button"
            title="Rafraîchir tout"
            aria-label="Rafraîchir tout"
            onClick={onRefresh}
            className={cn(
              "rounded-md p-1.5 text-(--color-muted) transition-colors hover:bg-(--color-panel-hover) hover:text-(--color-text)",
              busy && "animate-spin text-(--color-accent)",
            )}
          >
            <RefreshCw className="h-4 w-4" />
          </button>
        </div>
      </div>
    </div>
  );
}

const TONE: Record<string, string> = {
  danger: "text-(--color-danger)",
  warning: "text-(--color-warning)",
  success: "text-(--color-success)",
  accent: "text-(--color-accent)",
};

function Counter({ value, label, tone }: { value: number; label: string; tone: string }) {
  return (
    <span className="flex items-center gap-1">
      <span className={TONE[tone]}>{value}</span>
      <span className="text-(--color-muted-soft)">{label}</span>
    </span>
  );
}
