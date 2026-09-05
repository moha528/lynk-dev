import { Play, Plus, RotateCw, Square, Trash2, X } from "lucide-react";

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
import { cn } from "@/lib/utils";

import { COUNTER_ORDER, STATUS_LABEL, TONE_BG, TONE_TEXT, statusTone } from "../status";
import type { DevProfile, ServiceRuntime } from "../types";

type Props = {
  profiles: DevProfile[];
  activeProfileId: string | null;
  runtimes: ServiceRuntime[];
  selection: string[];
  onSelectProfile: (profileId: string) => void;
  onNewProfile: () => void;
  onDeleteProfile: () => void;
  onClearSelection: () => void;
  onStart: (serviceIds: string[]) => void;
  onStop: (serviceIds: string[]) => void;
  onRestart: (serviceIds: string[]) => void;
};

/**
 * Barre de pilotage : profil courant, compteurs d'état, actions groupées.
 *
 * Les actions portent sur la **sélection** dès qu'il y en a une, sur tous les
 * services sinon. Un seul jeu de boutons, dont la portée est annoncée à côté —
 * plutôt que deux barres qui se ressemblent.
 */
export function ProfileBar({
  profiles,
  activeProfileId,
  runtimes,
  selection,
  onSelectProfile,
  onNewProfile,
  onDeleteProfile,
  onClearSelection,
  onStart,
  onStop,
  onRestart,
}: Props) {
  const targets = selection.length > 0 ? selection : runtimes.map((runtime) => runtime.id);
  const counters = COUNTER_ORDER.map((status) => ({
    status,
    count: runtimes.filter((runtime) => runtime.status === status).length,
  })).filter((entry) => entry.count > 0);

  return (
    <div className="flex flex-wrap items-center gap-2 border-b border-(--color-border) bg-(--color-panel) px-3 py-2">
      <select
        value={activeProfileId ?? ""}
        onChange={(event) => onSelectProfile(event.target.value)}
        className="h-8 max-w-56 rounded-md border border-(--color-border) bg-(--color-bg) px-2 text-xs text-(--color-text) focus-visible:border-(--color-accent) focus-visible:outline-none"
      >
        {profiles.length === 0 && <option value="">Aucun profil</option>}
        {profiles.map((profile) => (
          <option key={profile.id} value={profile.id}>
            {profile.name}
          </option>
        ))}
      </select>

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
              {runtimes.length} service{runtimes.length > 1 ? "s" : ""} configuré
              {runtimes.length > 1 ? "s" : ""} seront perdus. Les services en marche ne sont pas
              arrêtés.
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

      <div className="flex items-center gap-3 pl-1">
        {counters.map((entry) => (
          <span key={entry.status} className="flex items-center gap-1.5 text-[11px]">
            <span className={cn("h-1.5 w-1.5 rounded-full", TONE_BG[statusTone(entry.status)])} />
            <span className={TONE_TEXT[statusTone(entry.status)]}>{entry.count}</span>
            <span className="text-(--color-muted-soft)">{STATUS_LABEL[entry.status]}</span>
          </span>
        ))}
        <span className="text-[11px] text-(--color-muted-soft)">{runtimes.length} au total</span>
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
            disabled={targets.length === 0}
            onClick={() => onStart(targets)}
          >
            <Play className="h-3.5 w-3.5" />
            Démarrer
          </Button>
          <Button
            size="sm"
            variant="outline"
            disabled={targets.length === 0}
            onClick={() => onStop(targets)}
          >
            <Square className="h-3.5 w-3.5" />
            Arrêter
          </Button>
          <Button
            size="sm"
            variant="outline"
            disabled={targets.length === 0}
            onClick={() => onRestart(targets)}
          >
            <RotateCw className="h-3.5 w-3.5" />
            Redémarrer
          </Button>
        </div>
      </div>
    </div>
  );
}
