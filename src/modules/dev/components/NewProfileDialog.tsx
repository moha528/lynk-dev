import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { FolderSearch, Loader2 } from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/Button";
import { Checkbox } from "@/components/ui/Checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/Dialog";
import { Input } from "@/components/ui/Input";
import { formatError, toastError } from "@/lib/feedback";

import { TYPE_LABEL } from "../status";
import { useDevStore } from "../store";
import type { DevProfile, ServiceConfig, ServiceScanResult } from "../types";

type Props = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

/**
 * Création d'un profil : choisir une racine, l'analyser, cocher ce qu'on garde.
 *
 * Tout se joue dans une seule fenêtre plutôt qu'un assistant en trois étapes —
 * il n'y a que deux décisions à prendre, un enchaînement d'écrans ne les rendait
 * pas plus claires.
 */
export function NewProfileDialog({ open, onOpenChange }: Props) {
  const scan = useDevStore((s) => s.scan);
  const scanning = useDevStore((s) => s.scanning);
  const progress = useDevStore((s) => s.scanProgress);
  const saveProfile = useDevStore((s) => s.saveProfile);
  const selectProfile = useDevStore((s) => s.selectProfile);

  const [name, setName] = useState("");
  const [rootPath, setRootPath] = useState("");
  const [results, setResults] = useState<ServiceScanResult[] | null>(null);
  const [kept, setKept] = useState<Set<string>>(new Set());
  const [saving, setSaving] = useState(false);

  const reset = () => {
    setName("");
    setRootPath("");
    setResults(null);
    setKept(new Set());
    setSaving(false);
  };

  const pickFolder = async () => {
    try {
      const picked = await openDialog({ directory: true, multiple: false });
      if (typeof picked !== "string") return;
      setRootPath(picked);
      setResults(null);
      if (!name) setName(picked.split(/[\\/]/).filter(Boolean).pop() ?? "");
    } catch (error) {
      toastError(formatError(error));
    }
  };

  const runScan = async () => {
    if (!rootPath) return;
    try {
      const found = await scan(rootPath);
      setResults(found);
      setKept(new Set(found.map((entry) => entry.workingDir)));
    } catch (error) {
      toastError(formatError(error));
    }
  };

  const create = async () => {
    const selected = (results ?? []).filter((entry) => kept.has(entry.workingDir));
    const profile: DevProfile = {
      id: crypto.randomUUID(),
      name: name.trim() || "Profil",
      rootPath,
      services: selected.map(toServiceConfig),
      createdAt: Date.now(),
    };
    setSaving(true);
    try {
      await saveProfile(profile);
      await selectProfile(profile.id);
      onOpenChange(false);
      reset();
    } catch (error) {
      toastError(formatError(error));
      setSaving(false);
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(value) => {
        onOpenChange(value);
        if (!value) reset();
      }}
    >
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Nouveau profil</DialogTitle>
          <DialogDescription>
            {results
              ? `${results.length} service${results.length > 1 ? "s" : ""} détecté${results.length > 1 ? "s" : ""}`
              : "Choisissez la racine à analyser."}
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-3">
          <Input
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Nom du profil"
          />

          <div className="flex gap-2">
            <Input
              value={rootPath}
              onChange={(event) => setRootPath(event.target.value)}
              placeholder="Racine des dépôts"
              className="font-mono text-xs"
            />
            <Button variant="outline" size="sm" onClick={() => void pickFolder()}>
              <FolderSearch className="h-3.5 w-3.5" />
              Parcourir
            </Button>
          </div>

          {scanning && (
            <p className="flex items-center gap-2 font-mono text-[11px] text-(--color-muted)">
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
              <span className="min-w-0 truncate">{progress?.current ?? rootPath}</span>
              <span className="ml-auto shrink-0">{progress?.found ?? 0}</span>
            </p>
          )}

          {results && !scanning && (
            <div className="max-h-64 overflow-y-auto rounded-md border border-(--color-border)">
              {results.length === 0 ? (
                <p className="p-4 text-center text-xs text-(--color-muted)">
                  Rien de reconnaissable sous cette racine.
                </p>
              ) : (
                results.map((entry) => (
                  <div
                    key={entry.workingDir}
                    className="flex items-center gap-2 border-b border-(--color-border) px-2 py-1.5 last:border-b-0 hover:bg-(--color-panel-hover)"
                  >
                    <Checkbox
                      className="min-w-0 flex-1"
                      label={<span className="block truncate">{entry.name}</span>}
                      checked={kept.has(entry.workingDir)}
                      onCheckedChange={(value) =>
                        setKept((current) => {
                          const next = new Set(current);
                          if (value) next.add(entry.workingDir);
                          else next.delete(entry.workingDir);
                          return next;
                        })
                      }
                    />
                    <span className="shrink-0 rounded bg-(--color-panel) px-1.5 py-0.5 text-[10px] text-(--color-muted)">
                      {TYPE_LABEL[entry.type]}
                    </span>
                    {entry.suggestedPort && (
                      <span className="shrink-0 font-mono text-[10px] text-(--color-muted-soft)">
                        :{entry.suggestedPort}
                      </span>
                    )}
                  </div>
                ))
              )}
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="ghost" size="sm" onClick={() => onOpenChange(false)}>
            Annuler
          </Button>
          {results ? (
            <Button size="sm" onClick={() => void create()} disabled={saving || kept.size === 0}>
              Créer ({kept.size})
            </Button>
          ) : (
            <Button size="sm" onClick={() => void runScan()} disabled={!rootPath || scanning}>
              Analyser
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function toServiceConfig(entry: ServiceScanResult): ServiceConfig {
  return {
    id: crypto.randomUUID(),
    name: entry.name,
    type: entry.type,
    workingDir: entry.workingDir,
    command: entry.suggestedCommand,
    buildCommand: entry.suggestedBuildCommand,
    port: entry.suggestedPort,
    autoRestart: false,
  };
}
