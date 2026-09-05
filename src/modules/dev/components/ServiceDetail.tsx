import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { FolderOpen, Hammer, Pencil, Play, RotateCw, Square, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";

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
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { LogView } from "@/components/ui/LogView";
import { aiApi } from "@/lib/ai";
import { formatError, toastError } from "@/lib/feedback";
import { cn } from "@/lib/utils";

import { STATUS_LABEL, TONE_TEXT, TYPE_LABEL, formatUptime, isUp, statusTone } from "../status";
import type { ServiceRuntime } from "../types";
import { StatusDot } from "./StatusDot";

type Props = {
  runtime: ServiceRuntime;
  onStart: () => void;
  onStop: () => void;
  onRestart: () => void;
  onBuild: () => void;
  onClearLogs: () => void;
  onEdit: () => void;
  onDelete: () => void;
};

type Tab = "logs" | "config" | "env";

const TABS: { id: Tab; label: string }[] = [
  { id: "logs", label: "Logs" },
  { id: "config", label: "Configuration" },
  { id: "env", label: "Environnement" },
];

export function ServiceDetail({
  runtime,
  onStart,
  onStop,
  onRestart,
  onBuild,
  onClearLogs,
  onEdit,
  onDelete,
}: Props) {
  const [tab, setTab] = useState<Tab>("logs");
  // La valeur n'est jamais lue : seul le rendu qu'elle déclenche compte.
  const [, setTick] = useState(0);

  useEffect(() => {
    const id = window.setInterval(() => setTick((t) => t + 1), 1_000);
    return () => window.clearInterval(id);
  }, []);

  const up = isUp(runtime);
  const tone = statusTone(runtime.status);
  const uptime =
    runtime.status === "running" && runtime.startedAt ? formatUptime(runtime.startedAt) : "";

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-(--color-border) px-3 py-2">
        <StatusDot status={runtime.status} />
        <h2 className="min-w-0 truncate text-sm font-semibold text-(--color-text)">
          {runtime.config.name}
        </h2>
        <Badge>{TYPE_LABEL[runtime.config.type]}</Badge>
        {runtime.config.port && (
          <span className="font-mono text-[11px] text-(--color-muted)">:{runtime.config.port}</span>
        )}
        <span className={cn("text-[11px]", TONE_TEXT[tone])}>{STATUS_LABEL[runtime.status]}</span>
        {uptime && <span className="font-mono text-[11px] text-(--color-muted)">{uptime}</span>}
        {runtime.pid && (
          <span className="font-mono text-[10px] text-(--color-muted-soft)">pid {runtime.pid}</span>
        )}

        <div className="ml-auto flex items-center gap-1">
          <Button size="sm" variant="outline" onClick={onStart} disabled={up}>
            <Play className="h-3.5 w-3.5" />
            Démarrer
          </Button>
          <Button size="sm" variant="outline" onClick={onStop} disabled={!up}>
            <Square className="h-3.5 w-3.5" />
            Arrêter
          </Button>
          <Button size="sm" variant="outline" onClick={onRestart}>
            <RotateCw className="h-3.5 w-3.5" />
            Redémarrer
          </Button>
          {runtime.config.buildCommand && (
            <Button size="sm" variant="ghost" onClick={onBuild} title={runtime.config.buildCommand}>
              <Hammer className="h-3.5 w-3.5" />
              Build
            </Button>
          )}
        </div>
      </header>

      {runtime.error && (
        <p className="border-b border-(--color-border) bg-(--color-panel) px-3 py-1.5 font-mono text-[11px] text-(--color-danger)">
          {runtime.error}
          {runtime.stuck && " — bloqué après 5 tentatives, redémarrez à la main"}
        </p>
      )}

      {runtime.status === "waiting" && runtime.waitingFor?.length ? (
        <p className="border-b border-(--color-border) bg-(--color-panel) px-3 py-1.5 text-[11px] text-(--color-warning)">
          En attente de {runtime.waitingFor.join(", ")}
        </p>
      ) : null}

      {runtime.status === "external" && (
        <p className="border-b border-(--color-border) bg-(--color-panel) px-3 py-1.5 text-[11px] text-(--color-accent)">
          Démarré hors de Lynk Dev — pas de logs, l'arrêt passera par le port.
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

      {tab === "logs" && (
        <LogView
          lines={runtime.logs}
          onClear={onClearLogs}
          onSummarize={async (logs) => {
            try {
              return (await aiApi.summarizeLogs(logs)).text;
            } catch (error) {
              toastError(formatError(error));
              throw error;
            }
          }}
        />
      )}
      {tab === "config" && <ConfigTab runtime={runtime} onEdit={onEdit} onDelete={onDelete} />}
      {tab === "env" && <EnvTab runtime={runtime} />}
    </div>
  );
}

function ConfigTab({
  runtime,
  onEdit,
  onDelete,
}: {
  runtime: ServiceRuntime;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const { config } = runtime;
  return (
    <div className="min-h-0 flex-1 overflow-y-auto p-3">
      <div className="flex items-center gap-2 pb-3">
        <Button size="sm" variant="outline" onClick={onEdit}>
          <Pencil className="h-3.5 w-3.5" />
          Modifier
        </Button>
        <AlertDialog>
          <AlertDialogTrigger asChild>
            <Button size="sm" variant="ghost">
              <Trash2 className="h-3.5 w-3.5" />
              Retirer
            </Button>
          </AlertDialogTrigger>
          <AlertDialogContent>
            <AlertDialogTitle>Retirer « {config.name} » du profil ?</AlertDialogTitle>
            <AlertDialogDescription>
              Seule la configuration est supprimée. Le dossier du service n'est pas touché, et un
              process en marche n'est pas arrêté.
            </AlertDialogDescription>
            <AlertDialogFooter>
              <AlertDialogCancel asChild>
                <Button variant="outline" size="sm">
                  Annuler
                </Button>
              </AlertDialogCancel>
              <AlertDialogAction asChild>
                <Button variant="destructive" size="sm" onClick={onDelete}>
                  Retirer
                </Button>
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </div>

      <dl className="grid grid-cols-[9rem_1fr] gap-x-4 gap-y-2 text-xs">
        <Field label="Commande" value={config.command} mono />
        <Field label="Répertoire" value={config.workingDir} mono>
          <button
            type="button"
            title="Ouvrir dans l'explorateur"
            onClick={() => {
              // Un clic qui ne fait rien du tout laisse croire à un bug ; le
              // cas réel est un répertoire déplacé ou supprimé.
              void revealItemInDir(config.workingDir).catch((error) =>
                toastError(formatError(error)),
              );
            }}
            className="ml-1.5 inline-flex text-(--color-muted) hover:text-(--color-accent)"
          >
            <FolderOpen className="h-3.5 w-3.5" />
          </button>
        </Field>
        {config.buildCommand && <Field label="Build" value={config.buildCommand} mono />}
        {config.port !== undefined && <Field label="Port" value={String(config.port)} mono />}
        {config.healthCheckUrl && <Field label="Santé" value={config.healthCheckUrl} mono />}
        {config.group && <Field label="Groupe" value={config.group} />}
        {config.dependsOn?.length ? (
          <Field label="Dépend de" value={config.dependsOn.join(", ")} />
        ) : null}
        <Field label="Redémarrage auto" value={config.autoRestart ? "oui" : "non"} />
      </dl>
    </div>
  );
}

function EnvTab({ runtime }: { runtime: ServiceRuntime }) {
  const entries = Object.entries(runtime.config.envVars ?? {});
  if (entries.length === 0) {
    return <p className="p-6 text-center text-xs text-(--color-muted)">Aucune variable</p>;
  }
  return (
    <div className="min-h-0 flex-1 overflow-y-auto p-3">
      <dl className="grid grid-cols-[minmax(8rem,auto)_1fr] gap-x-4 gap-y-1.5 font-mono text-[11px]">
        {entries.map(([key, value]) => (
          <div key={key} className="contents">
            <dt className="truncate text-(--color-muted)">{key}</dt>
            <dd className="break-all text-(--color-text-soft)">{value}</dd>
          </div>
        ))}
      </dl>
    </div>
  );
}

function Field({
  label,
  value,
  mono,
  children,
}: {
  label: string;
  value: string;
  mono?: boolean;
  children?: React.ReactNode;
}) {
  return (
    <div className="contents">
      <dt className="text-(--color-muted)">{label}</dt>
      <dd
        className={cn(
          "flex min-w-0 items-center break-all text-(--color-text-soft)",
          mono && "font-mono text-[11px]",
        )}
      >
        <span className="min-w-0">{value}</span>
        {children}
      </dd>
    </div>
  );
}
