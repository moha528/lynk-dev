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

import { gitApi } from "../ipc";
import { useGitStore } from "../store";
import type { GitProfile, RepoScanResult } from "../types";

type Props = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

/** Choisir une racine, l'analyser, cocher les dépôts à suivre. */
export function NewGitProfileDialog({ open, onOpenChange }: Props) {
  const saveProfile = useGitStore((s) => s.saveProfile);
  const selectProfile = useGitStore((s) => s.selectProfile);

  const [name, setName] = useState("");
  const [rootPath, setRootPath] = useState("");
  const [scanning, setScanning] = useState(false);
  const [results, setResults] = useState<RepoScanResult[] | null>(null);
  const [kept, setKept] = useState<Set<string>>(new Set());
  const [saving, setSaving] = useState(false);

  const reset = () => {
    setName("");
    setRootPath("");
    setResults(null);
    setKept(new Set());
    setScanning(false);
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
    setScanning(true);
    try {
      const found = await gitApi.scanRepos(rootPath);
      setResults(found);
      setKept(new Set(found.map((entry) => entry.path)));
    } catch (error) {
      toastError(formatError(error));
    } finally {
      setScanning(false);
    }
  };

  const create = async () => {
    const profile: GitProfile = {
      id: crypto.randomUUID(),
      name: name.trim() || "Profil",
      rootPath,
      repoPaths: (results ?? []).filter((entry) => kept.has(entry.path)).map((entry) => entry.path),
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
              ? `${results.length} dépôt${results.length > 1 ? "s" : ""} trouvé${results.length > 1 ? "s" : ""}`
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
            <p className="flex items-center gap-2 text-[11px] text-(--color-muted)">
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
              Analyse en cours
            </p>
          )}

          {results && !scanning && (
            <div className="max-h-64 overflow-y-auto rounded-md border border-(--color-border)">
              {results.length === 0 ? (
                <p className="p-4 text-center text-xs text-(--color-muted)">
                  Aucun dépôt Git sous cette racine.
                </p>
              ) : (
                results.map((entry) => (
                  <div
                    key={entry.path}
                    className="flex items-center gap-2 border-b border-(--color-border) px-2 py-1.5 last:border-b-0 hover:bg-(--color-panel-hover)"
                  >
                    <Checkbox
                      className="min-w-0 flex-1"
                      label={<span className="block truncate">{entry.name}</span>}
                      checked={kept.has(entry.path)}
                      onCheckedChange={(value) =>
                        setKept((current) => {
                          const next = new Set(current);
                          if (value) next.add(entry.path);
                          else next.delete(entry.path);
                          return next;
                        })
                      }
                    />
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
