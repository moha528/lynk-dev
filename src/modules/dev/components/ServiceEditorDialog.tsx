import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { FolderSearch, Plus, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { Button } from "@/components/ui/Button";
import { Checkbox } from "@/components/ui/Checkbox";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/Dialog";
import { Input } from "@/components/ui/Input";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/Select";
import { formatError, toastError } from "@/lib/feedback";
import { cn } from "@/lib/utils";

import { TYPE_LABEL } from "../status";
import { useDevStore } from "../store";
import type { ServiceConfig, ServiceType } from "../types";

type Props = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** `null` pour créer un service, sinon celui qu'on modifie. */
  service: ServiceConfig | null;
};

/**
 * Les 22 familles, groupees par ecosysteme.
 *
 * A plat, une liste de cette longueur est illisible : on cherche « Next » sans
 * savoir s'il est range avec Node ou avec Vite. Les groupes rendent le
 * balayage immediat.
 */
const TYPE_GROUPS: { label: string; types: ServiceType[] }[] = [
  { label: "JVM", types: ["spring-boot-maven", "spring-boot-gradle"] },
  {
    label: "JavaScript",
    types: ["next", "nuxt", "angular", "nest", "svelte-kit", "astro", "remix", "vite", "node"],
  },
  { label: "Python", types: ["django", "fastapi", "flask", "python"] },
  { label: "Autres", types: ["go", "rust", "dotnet", "laravel", "rails"] },
  { label: "Conteneurs", types: ["docker-compose", "custom"] },
];

type EnvRow = { key: string; value: string };

function blank(): ServiceConfig {
  return {
    id: crypto.randomUUID(),
    name: "",
    type: "custom",
    workingDir: "",
    command: "",
    autoRestart: false,
  };
}

/**
 * Création et modification d'un service.
 *
 * L'écran d'origine séparait « créer » et « modifier » en deux assistants
 * distincts alors que les champs sont les mêmes : une seule fenêtre, dont seul
 * le titre change.
 */
export function ServiceEditorDialog({ open, onOpenChange, service }: Props) {
  const saveService = useDevStore((s) => s.saveService);
  const runtimes = useDevStore((s) => s.runtimes);

  const [draft, setDraft] = useState<ServiceConfig>(blank);
  const [env, setEnv] = useState<EnvRow[]>([]);
  const [saving, setSaving] = useState(false);

  // Réinitialise à chaque ouverture : sans ça, rouvrir la fenêtre sur un autre
  // service afficherait encore les champs du précédent.
  useEffect(() => {
    if (!open) return;
    const base = service ?? blank();
    setDraft(base);
    setEnv(Object.entries(base.envVars ?? {}).map(([key, value]) => ({ key, value })));
  }, [open, service]);

  const others = useMemo(
    () => Object.values(runtimes).filter((runtime) => runtime.id !== draft.id),
    [runtimes, draft.id],
  );

  const set = <K extends keyof ServiceConfig>(key: K, value: ServiceConfig[K]) => {
    setDraft((current) => ({ ...current, [key]: value }));
  };

  const pickFolder = async () => {
    try {
      const picked = await openDialog({ directory: true, multiple: false });
      if (typeof picked !== "string") return;
      set("workingDir", picked);
      if (!draft.name) set("name", picked.split(/[\\/]/).filter(Boolean).pop() ?? "");
    } catch (error) {
      toastError(formatError(error));
    }
  };

  const submit = async () => {
    const envVars = Object.fromEntries(
      env.filter((row) => row.key.trim()).map((row) => [row.key.trim(), row.value]),
    );
    const payload: ServiceConfig = {
      ...draft,
      name: draft.name.trim(),
      workingDir: draft.workingDir.trim(),
      command: draft.command.trim(),
      buildCommand: draft.buildCommand?.trim() || undefined,
      healthCheckUrl: draft.healthCheckUrl?.trim() || undefined,
      group: draft.group?.trim() || undefined,
      dependsOn: draft.dependsOn?.length ? draft.dependsOn : undefined,
      envVars: Object.keys(envVars).length > 0 ? envVars : undefined,
    };

    setSaving(true);
    try {
      await saveService(payload);
      onOpenChange(false);
    } catch (error) {
      toastError(formatError(error));
    } finally {
      setSaving(false);
    }
  };

  const valid = draft.name.trim() && draft.workingDir.trim() && draft.command.trim();

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[85vh] max-w-2xl overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{service ? "Modifier le service" : "Nouveau service"}</DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-3">
          <Row label="Nom">
            <Input
              value={draft.name}
              onChange={(event) => set("name", event.target.value)}
              className="h-8 text-xs"
            />
          </Row>

          <Row label="Type">
            <Select value={draft.type} onValueChange={(value) => set("type", value as ServiceType)}>
              <SelectTrigger aria-label="Type de service">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {TYPE_GROUPS.map((group) => (
                  <SelectGroup key={group.label} label={group.label}>
                    {group.types.map((type) => (
                      <SelectItem key={type} value={type}>
                        {TYPE_LABEL[type]}
                      </SelectItem>
                    ))}
                  </SelectGroup>
                ))}
              </SelectContent>
            </Select>
          </Row>

          <Row label="Répertoire">
            <div className="flex gap-2">
              <Input
                value={draft.workingDir}
                onChange={(event) => set("workingDir", event.target.value)}
                className="h-8 font-mono text-xs"
              />
              <Button variant="outline" size="sm" onClick={() => void pickFolder()}>
                <FolderSearch className="h-3.5 w-3.5" />
              </Button>
            </div>
          </Row>

          <Row label="Commande">
            <Input
              value={draft.command}
              onChange={(event) => set("command", event.target.value)}
              className="h-8 font-mono text-xs"
            />
          </Row>

          <Row label="Build">
            <Input
              value={draft.buildCommand ?? ""}
              onChange={(event) => set("buildCommand", event.target.value)}
              className="h-8 font-mono text-xs"
            />
          </Row>

          <div className="grid grid-cols-2 gap-3">
            <Row label="Port">
              <Input
                value={draft.port === undefined ? "" : String(draft.port)}
                onChange={(event) => {
                  const parsed = Number.parseInt(event.target.value, 10);
                  set("port", Number.isNaN(parsed) ? undefined : parsed);
                }}
                inputMode="numeric"
                className="h-8 font-mono text-xs"
              />
            </Row>
            <Row label="Groupe">
              <Input
                value={draft.group ?? ""}
                onChange={(event) => set("group", event.target.value)}
                className="h-8 text-xs"
              />
            </Row>
          </div>

          <Row label="URL de santé">
            <Input
              value={draft.healthCheckUrl ?? ""}
              onChange={(event) => set("healthCheckUrl", event.target.value)}
              className="h-8 font-mono text-xs"
            />
          </Row>

          <Row label="Dépend de">
            {others.length === 0 ? (
              <p className="text-[11px] text-(--color-muted-soft)">Aucun autre service</p>
            ) : (
              <div className="flex flex-wrap gap-x-4 gap-y-1">
                {others.map((runtime) => (
                  <Checkbox
                    key={runtime.id}
                    label={runtime.config.name}
                    checked={draft.dependsOn?.includes(runtime.id) ?? false}
                    onCheckedChange={(value) => {
                      const current = new Set(draft.dependsOn ?? []);
                      if (value) current.add(runtime.id);
                      else current.delete(runtime.id);
                      set("dependsOn", [...current]);
                    }}
                  />
                ))}
              </div>
            )}
          </Row>

          <Row label="Environnement">
            <div className="flex flex-col gap-1">
              {env.map((row, index) => (
                <div key={`env-${index}-${row.key}`} className="flex gap-2">
                  <Input
                    value={row.key}
                    onChange={(event) =>
                      setEnv((rows) =>
                        rows.map((r, i) => (i === index ? { ...r, key: event.target.value } : r)),
                      )
                    }
                    placeholder="CLÉ"
                    className="h-8 w-48 font-mono text-xs"
                  />
                  <Input
                    value={row.value}
                    onChange={(event) =>
                      setEnv((rows) =>
                        rows.map((r, i) => (i === index ? { ...r, value: event.target.value } : r)),
                      )
                    }
                    placeholder="valeur"
                    className="h-8 flex-1 font-mono text-xs"
                  />
                  <button
                    type="button"
                    aria-label="Retirer la variable"
                    onClick={() => setEnv((rows) => rows.filter((_, i) => i !== index))}
                    className="rounded p-1 text-(--color-muted) transition-colors hover:text-(--color-danger)"
                  >
                    <X className="h-3.5 w-3.5" />
                  </button>
                </div>
              ))}
              <button
                type="button"
                onClick={() => setEnv((rows) => [...rows, { key: "", value: "" }])}
                className="inline-flex w-fit items-center gap-1 text-[11px] text-(--color-muted) hover:text-(--color-accent)"
              >
                <Plus className="h-3 w-3" />
                ajouter
              </button>
            </div>
          </Row>

          <Checkbox
            label="Redémarrage automatique après un crash"
            checked={draft.autoRestart}
            onCheckedChange={(value) => set("autoRestart", value)}
          />
        </div>

        <DialogFooter>
          <Button variant="ghost" size="sm" onClick={() => onOpenChange(false)}>
            Annuler
          </Button>
          <Button size="sm" disabled={saving || !valid} onClick={() => void submit()}>
            Enregistrer
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function Row({
  label,
  children,
  className,
}: {
  label: string;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("flex flex-col gap-1", className)}>
      <span className="text-[10px] font-semibold uppercase tracking-wider text-(--color-muted)">
        {label}
      </span>
      {children}
    </div>
  );
}
