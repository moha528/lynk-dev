/**
 * Contrat du Dev Manager — miroir exact de `src-tauri/src/dev/types.rs`.
 *
 * Le backend sérialise en camelCase : ces types doivent rester alignés champ
 * pour champ. Une divergence ne casse pas la compilation, elle produit des
 * `undefined` silencieux à l'exécution — d'où les tests de round-trip côté Rust.
 */

/**
 * Famille d'un service détecté — miroir de `ServiceType` côté Rust.
 *
 * ⚠️ Seul `docker-compose` change un comportement (sondes et arrêt passent par
 * `docker compose`). Les autres sont cosmétiques : ils nomment ce qui a été
 * reconnu et justifient la commande proposée.
 */
export type ServiceType =
  // JVM
  | "spring-boot-maven"
  | "spring-boot-gradle"
  // JavaScript / TypeScript
  | "next"
  | "nuxt"
  | "angular"
  | "nest"
  | "svelte-kit"
  | "astro"
  | "remix"
  | "vite"
  | "node"
  // Python
  | "django"
  | "fastapi"
  | "flask"
  | "python"
  // Autres écosystèmes
  | "go"
  | "rust"
  | "dotnet"
  | "laravel"
  | "rails"
  // Conteneurs
  | "docker-compose"
  | "custom";

export type ServiceStatus =
  | "stopped"
  | "starting"
  | "running"
  | "stopping"
  | "error"
  /** Détecté sur son port, mais démarré hors de Lynk Dev. */
  | "external"
  /** En attente de ses dépendances lors d'un démarrage groupé. */
  | "waiting";

export type ExitReason = "normal" | "crash" | "killed";

export type LogStream = "stdout" | "stderr" | "system";

export interface LogEntry {
  /** Millisecondes depuis l'époque Unix. */
  timestamp: number;
  stream: LogStream;
  text: string;
}

export interface ServiceConfig {
  id: string;
  name: string;
  type: ServiceType;
  workingDir: string;
  command: string;
  buildCommand?: string;
  port?: number;
  healthCheckUrl?: string;
  group?: string;
  dependsOn?: string[];
  envVars?: Record<string, string>;
  autoRestart: boolean;
}

export interface DevProfile {
  id: string;
  name: string;
  rootPath: string;
  services: ServiceConfig[];
  /** Millisecondes depuis l'époque Unix. */
  createdAt: number;
}

/** Charge utile de l'événement `dev:service:status`. */
export interface StatusUpdate {
  serviceId: string;
  status: ServiceStatus;
  pid?: number;
  error?: string;
  exitReason?: ExitReason;
  exitCode?: number;
  retryCount?: number;
  /** A épuisé ses redémarrages automatiques. */
  stuck?: boolean;
  /** Noms des dépendances encore attendues. */
  waitingFor?: string[];
}

/** Charge utile de l'événement `dev:service:log`. */
export interface LogEvent {
  serviceId: string;
  entry: LogEntry;
}

/** Charge utile de l'événement `dev:scan:progress`. */
export interface ScanProgress {
  current: string;
  scanned: number;
  found: number;
}

export interface ServiceScanResult {
  name: string;
  type: ServiceType;
  workingDir: string;
  suggestedCommand: string;
  suggestedBuildCommand?: string;
  suggestedPort?: number;
}

export interface PortRequest {
  serviceId: string;
  port: number;
}

export interface PortCheckResult {
  serviceId: string;
  port: number;
  available: boolean;
}

export interface ManagedProcessInfo {
  serviceId: string;
  pid: number;
  startedAt: number;
}

export interface ProbeResult {
  serviceId: string;
  detected: boolean;
  /** Confirmé par une URL de santé ou `docker compose ps`, pas juste un port pris. */
  viaHealthCheck: boolean;
}

export interface DockerContainer {
  name: string;
  state: string;
  health: string;
}

export type DockerHealth = "up" | "partial" | "down";

export interface DockerHealthReport {
  status: DockerHealth;
  services: DockerContainer[];
}

/**
 * État d'exécution d'un service, tel que l'écran le manipule.
 *
 * `config` + ce que le backend a annoncé depuis. Les logs sont bornés côté
 * store : un service bavard remplirait la mémoire en quelques minutes.
 */
export interface ServiceRuntime {
  id: string;
  config: ServiceConfig;
  status: ServiceStatus;
  pid?: number;
  logs: LogEntry[];
  error?: string;
  startedAt?: number;
  portAvailable?: boolean;
  exitReason?: ExitReason;
  exitCode?: number;
  retryCount?: number;
  stuck?: boolean;
  waitingFor?: string[];
}
