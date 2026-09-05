import { Boxes } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { Button } from "@/components/ui/Button";
import { formatError, toastError } from "@/lib/feedback";
import type { CheckOptions } from "@/lib/selection";
import { rangeBetween } from "@/lib/selection";

import { NewProfileDialog } from "./components/NewProfileDialog";
import { ProfileBar } from "./components/ProfileBar";
import { ServiceDetail } from "./components/ServiceDetail";
import { ServiceEditorDialog } from "./components/ServiceEditorDialog";
import { ServiceList } from "./components/ServiceList";
import { useDevStore } from "./store";
import type { ServiceConfig } from "./types";

/**
 * Dev Manager — écran principal.
 *
 * Trois zones : la barre de pilotage (profil, compteurs, actions groupées), la
 * liste des services, et le détail du service courant.
 */
export function DevManagerView() {
  const hydrate = useDevStore((s) => s.hydrate);
  const subscribe = useDevStore((s) => s.subscribe);
  const profiles = useDevStore((s) => s.profiles);
  const activeProfileId = useDevStore((s) => s.activeProfileId);
  const runtimeMap = useDevStore((s) => s.runtimes);
  const selectedServiceId = useDevStore((s) => s.selectedServiceId);
  const selectService = useDevStore((s) => s.selectService);
  const selectProfile = useDevStore((s) => s.selectProfile);
  const deleteProfile = useDevStore((s) => s.deleteProfile);
  const clearLogs = useDevStore((s) => s.clearLogs);
  const start = useDevStore((s) => s.start);
  const stop = useDevStore((s) => s.stop);
  const restart = useDevStore((s) => s.restart);
  const build = useDevStore((s) => s.build);
  const startMany = useDevStore((s) => s.startMany);
  const stopMany = useDevStore((s) => s.stopMany);
  const restartMany = useDevStore((s) => s.restartMany);
  const removeService = useDevStore((s) => s.removeService);

  const [checked, setChecked] = useState<Set<string>>(new Set());
  /** Dernière ligne cochée à la main : point de départ d'un Maj+clic. */
  const [anchor, setAnchor] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  /** `service: null` = création ; sinon on modifie celui-ci. */
  const [editing, setEditing] = useState<{ open: boolean; service: ServiceConfig | null }>({
    open: false,
    service: null,
  });

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  // L'abonnement au superviseur vit aussi longtemps que l'écran.
  useEffect(() => subscribe(), [subscribe]);

  const runtimes = useMemo(() => Object.values(runtimeMap), [runtimeMap]);
  const selected = selectedServiceId ? runtimeMap[selectedServiceId] : undefined;
  const selection = useMemo(
    () => runtimes.filter((runtime) => checked.has(runtime.id)).map((runtime) => runtime.id),
    [runtimes, checked],
  );

  const check = (serviceId: string, value: boolean, options: CheckOptions) => {
    // Maj+clic : toute la plage depuis l'ancre prend l'état de la case cliquée.
    const targets =
      options.shiftKey && anchor ? rangeBetween(options.ordered, anchor, serviceId) : [serviceId];
    setChecked((current) => {
      const next = new Set(current);
      for (const id of targets) {
        if (value) next.add(id);
        else next.delete(id);
      }
      return next;
    });
    // L'ancre ne bouge pas pendant un Maj+clic : on peut étendre la plage
    // plusieurs fois de suite depuis le même point de départ.
    if (!options.shiftKey) setAnchor(serviceId);
  };

  const checkMany = (serviceIds: string[], value: boolean) => {
    setChecked((current) => {
      const next = new Set(current);
      for (const id of serviceIds) {
        if (value) next.add(id);
        else next.delete(id);
      }
      return next;
    });
  };

  const guard = (action: Promise<void>) => {
    void action.catch((error) => toastError(formatError(error)));
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <ProfileBar
        profiles={profiles}
        activeProfileId={activeProfileId}
        runtimes={runtimes}
        selection={selection}
        onSelectProfile={(profileId) => {
          // Les identifiants cochés n'existent plus dans le nouveau profil :
          // les garder ferait porter les actions groupées sur du vide.
          setChecked(new Set());
          guard(selectProfile(profileId));
        }}
        onNewProfile={() => setCreating(true)}
        onDeleteProfile={() => {
          if (!activeProfileId) return;
          setChecked(new Set());
          guard(deleteProfile(activeProfileId));
        }}
        onClearSelection={() => {
          setChecked(new Set());
          setAnchor(null);
        }}
        onStart={(ids) => guard(startMany(ids))}
        onStop={(ids) => guard(stopMany(ids))}
        onRestart={(ids) => guard(restartMany(ids))}
      />

      {profiles.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-3">
          <Boxes className="h-8 w-8 text-(--color-muted-soft)" />
          <p className="text-sm text-(--color-muted)">Aucun profil</p>
          <Button size="sm" onClick={() => setCreating(true)}>
            Analyser un dossier
          </Button>
        </div>
      ) : (
        <div className="flex min-h-0 flex-1">
          <aside className="flex w-72 shrink-0 flex-col border-r border-(--color-border) bg-(--color-bg-soft)">
            <ServiceList
              runtimes={runtimes}
              selectedId={selectedServiceId}
              onSelect={selectService}
              checked={checked}
              onCheck={check}
              onCheckMany={checkMany}
              onAdd={() => setEditing({ open: true, service: null })}
            />
          </aside>

          {selected ? (
            <ServiceDetail
              key={selected.id}
              runtime={selected}
              onStart={() => guard(start(selected.id))}
              onStop={() => guard(stop(selected.id))}
              onRestart={() => guard(restart(selected.id))}
              onBuild={() => guard(build(selected.id))}
              onClearLogs={() => clearLogs(selected.id)}
              onEdit={() => setEditing({ open: true, service: selected.config })}
              onDelete={() => guard(removeService(selected.id))}
            />
          ) : (
            <div className="flex flex-1 items-center justify-center">
              <p className="text-xs text-(--color-muted)">Choisissez un service</p>
            </div>
          )}
        </div>
      )}

      <NewProfileDialog open={creating} onOpenChange={setCreating} />
      <ServiceEditorDialog
        open={editing.open}
        service={editing.service}
        onOpenChange={(open) => setEditing((current) => ({ ...current, open }))}
      />
    </div>
  );
}
