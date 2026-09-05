import type { ServiceRuntime, ServiceStatus, ServiceType } from "./types";

export const STATUS_LABEL: Record<ServiceStatus, string> = {
  stopped: "Arrêté",
  starting: "Démarrage",
  running: "En marche",
  stopping: "Arrêt",
  error: "Erreur",
  external: "Externe",
  waiting: "En attente",
};

export type Tone = "success" | "warning" | "danger" | "accent" | "muted";

export function statusTone(status: ServiceStatus): Tone {
  switch (status) {
    case "running":
      return "success";
    case "starting":
    case "stopping":
    case "waiting":
      return "warning";
    case "error":
      return "danger";
    case "external":
      return "accent";
    default:
      return "muted";
  }
}

/** Classe de couleur de texte pour un ton. */
export const TONE_TEXT: Record<Tone, string> = {
  success: "text-(--color-success)",
  warning: "text-(--color-warning)",
  danger: "text-(--color-danger)",
  accent: "text-(--color-accent)",
  muted: "text-(--color-muted)",
};

/** Classe de couleur de fond pour un ton (pastilles, jauges). */
export const TONE_BG: Record<Tone, string> = {
  success: "bg-(--color-success)",
  warning: "bg-(--color-warning)",
  danger: "bg-(--color-danger)",
  accent: "bg-(--color-accent)",
  muted: "bg-(--color-muted-soft)",
};

/** Un état transitoire mérite une animation : il va bouger tout seul. */
export function isTransient(status: ServiceStatus): boolean {
  return status === "starting" || status === "stopping" || status === "waiting";
}

export const TYPE_LABEL: Record<ServiceType, string> = {
  "spring-boot-maven": "Maven",
  "spring-boot-gradle": "Gradle",
  node: "Node",
  python: "Python",
  "docker-compose": "Compose",
  custom: "Custom",
};

/**
 * Durée écoulée, en une expression courte : `12s`, `4 min`, `1 h 20`, `3 j`.
 * Rendue vide sous la seconde pour éviter un `0s` qui clignote.
 */
export function formatUptime(startedAt: number, now = Date.now()): string {
  const seconds = Math.floor((now - startedAt) / 1000);
  if (seconds < 1) return "";
  if (seconds < 60) return `${seconds}s`;

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} min`;

  const hours = Math.floor(minutes / 60);
  if (hours < 24) {
    const rest = minutes % 60;
    return rest === 0 ? `${hours} h` : `${hours} h ${String(rest).padStart(2, "0")}`;
  }

  return `${Math.floor(hours / 24)} j`;
}

/** Un service qu'on peut arrêter : il tourne, d'une façon ou d'une autre. */
export function isUp(runtime: ServiceRuntime): boolean {
  return (
    runtime.status === "running" ||
    runtime.status === "starting" ||
    runtime.status === "external" ||
    runtime.status === "waiting"
  );
}

/** Ordre d'affichage des compteurs de la barre d'état. */
export const COUNTER_ORDER: ServiceStatus[] = ["running", "starting", "error", "external"];
