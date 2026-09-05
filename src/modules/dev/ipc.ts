import { invoke } from "@tauri-apps/api/core";
import { type UnlistenFn, listen } from "@tauri-apps/api/event";

import type {
  DevProfile,
  DockerHealthReport,
  LogEvent,
  ManagedProcessInfo,
  PortCheckResult,
  PortRequest,
  ProbeResult,
  ScanProgress,
  ServiceScanResult,
  StatusUpdate,
} from "./types";

/**
 * Surface IPC du Dev Manager.
 *
 * Un seul endroit connaît les noms de commandes et d'événements : les
 * composants et le store passent par ici, jamais par `invoke` en direct.
 */
export const devApi = {
  // ── Profils ──────────────────────────────────────────────────────────
  listProfiles: () => invoke<DevProfile[]>("dev_profile_list"),
  saveProfile: (profile: DevProfile) => invoke<void>("dev_profile_save", { profile }),
  deleteProfile: (profileId: string) => invoke<void>("dev_profile_delete", { profileId }),

  // ── Détection ────────────────────────────────────────────────────────
  scan: (rootPath: string) => invoke<ServiceScanResult[]>("dev_scan", { rootPath }),
  detect: (dirPath: string) => invoke<ServiceScanResult | null>("dev_detect", { dirPath }),

  // ── Cycle de vie ─────────────────────────────────────────────────────
  start: (profileId: string, serviceId: string) =>
    invoke<boolean>("dev_service_start", { profileId, serviceId }),
  stop: (profileId: string, serviceId: string) =>
    invoke<boolean>("dev_service_stop", { profileId, serviceId }),
  restart: (profileId: string, serviceId: string) =>
    invoke<boolean>("dev_service_restart", { profileId, serviceId }),
  build: (profileId: string, serviceId: string) =>
    invoke<boolean>("dev_service_build", { profileId, serviceId }),

  // ── Opérations groupées ──────────────────────────────────────────────
  startBatch: (profileId: string, serviceIds: string[]) =>
    invoke<boolean>("dev_service_start_batch", { profileId, serviceIds }),
  stopBatch: (profileId: string, serviceIds: string[]) =>
    invoke<boolean>("dev_service_stop_batch", { profileId, serviceIds }),
  restartBatch: (profileId: string, serviceIds: string[]) =>
    invoke<boolean>("dev_service_restart_batch", { profileId, serviceIds }),

  // ── Sondes ───────────────────────────────────────────────────────────
  checkPort: (port: number) => invoke<boolean>("dev_port_check", { port }),
  checkPorts: (ports: PortRequest[]) =>
    invoke<PortCheckResult[]>("dev_port_check_batch", { ports }),
  dockerHealth: (workingDir: string, composeFile?: string) =>
    invoke<DockerHealthReport>("dev_docker_health", { workingDir, composeFile }),
  probe: (profileId: string) => invoke<ProbeResult[]>("dev_service_probe", { profileId }),
  processList: (profileId: string) =>
    invoke<ManagedProcessInfo[]>("dev_process_list", { profileId }),
};

/**
 * Abonnements aux flux du superviseur.
 *
 * ⚠️ `listen` est **asynchrone** : la fonction de désabonnement n'existe qu'une
 * fois la promesse résolue. Un `useEffect` qui se démonte avant devrait sinon
 * laisser l'abonnement en place — d'où le drapeau `cancelled`.
 */
export function onServiceLog(handler: (event: LogEvent) => void): () => void {
  return subscribe<LogEvent>("dev:service:log", handler);
}

export function onServiceStatus(handler: (event: StatusUpdate) => void): () => void {
  return subscribe<StatusUpdate>("dev:service:status", handler);
}

export function onScanProgress(handler: (event: ScanProgress) => void): () => void {
  return subscribe<ScanProgress>("dev:scan:progress", handler);
}

function subscribe<T>(channel: string, handler: (payload: T) => void): () => void {
  let cancelled = false;
  let unlisten: UnlistenFn | undefined;

  void listen<T>(channel, (event) => handler(event.payload)).then((fn) => {
    if (cancelled) {
      fn();
      return;
    }
    unlisten = fn;
  });

  return () => {
    cancelled = true;
    unlisten?.();
  };
}
