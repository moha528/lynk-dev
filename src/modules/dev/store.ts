import { create } from "zustand";

import { useSettingsStore } from "@/stores/useSettingsStore";

import { devApi, onScanProgress, onServiceLog, onServiceStatus } from "./ipc";
import type {
  DevProfile,
  LogEntry,
  ScanProgress,
  ServiceConfig,
  ServiceRuntime,
  ServiceScanResult,
  StatusUpdate,
} from "./types";

/**
 * Lignes de log conservées **par service**.
 *
 * Sans plafond, un service bavard (un `mvn` en boucle de redémarrage) fait
 * grossir la mémoire de la fenêtre jusqu'à la rendre inutilisable. On garde une
 * fenêtre glissante : c'est ce qu'on regarde en pratique.
 */
const MAX_LOG_LINES = 5_000;

function emptyRuntime(config: ServiceConfig): ServiceRuntime {
  return { id: config.id, config, status: "stopped", logs: [] };
}

type DevState = {
  profiles: DevProfile[];
  activeProfileId: string | null;
  /** Indexé par `serviceId`, reconstruit à chaque changement de profil. */
  runtimes: Record<string, ServiceRuntime>;
  selectedServiceId: string | null;
  loading: boolean;
  scanning: boolean;
  scanProgress: ScanProgress | null;

  hydrate: () => Promise<void>;
  selectProfile: (profileId: string | null) => Promise<void>;
  selectService: (serviceId: string | null) => void;
  saveProfile: (profile: DevProfile) => Promise<void>;
  deleteProfile: (profileId: string) => Promise<void>;
  /** Crée ou remplace un service du profil actif, **sans perdre l'état vivant**. */
  saveService: (config: ServiceConfig) => Promise<void>;
  removeService: (serviceId: string) => Promise<void>;

  start: (serviceId: string) => Promise<void>;
  stop: (serviceId: string) => Promise<void>;
  restart: (serviceId: string) => Promise<void>;
  build: (serviceId: string) => Promise<void>;
  startMany: (serviceIds: string[]) => Promise<void>;
  stopMany: (serviceIds: string[]) => Promise<void>;
  restartMany: (serviceIds: string[]) => Promise<void>;

  scan: (rootPath: string) => Promise<ServiceScanResult[]>;
  /** Réaligne l'écran sur la réalité : process gérés + services externes. */
  reconcile: () => Promise<void>;
  clearLogs: (serviceId: string) => void;

  /** Branche les flux du superviseur. Rend la fonction de désabonnement. */
  subscribe: () => () => void;
};

export const useDevStore = create<DevState>((set, get) => ({
  profiles: [],
  activeProfileId: null,
  runtimes: {},
  selectedServiceId: null,
  loading: false,
  scanning: false,
  scanProgress: null,

  async hydrate() {
    set({ loading: true });
    try {
      const profiles = await devApi.listProfiles();
      // Le profil retenu de la session précédente, s'il existe encore.
      const remembered = useSettingsStore.getState().devProfileId;
      const known = profiles.some((profile) => profile.id === remembered) ? remembered : null;
      const activeProfileId = get().activeProfileId ?? known ?? profiles[0]?.id ?? null;
      set({ profiles, loading: false });
      await get().selectProfile(activeProfileId);
    } catch (error) {
      console.error("dev: chargement des profils", error);
      set({ loading: false });
    }
  },

  async selectProfile(profileId) {
    const profile = get().profiles.find((p) => p.id === profileId) ?? null;
    const runtimes: Record<string, ServiceRuntime> = {};
    for (const config of profile?.services ?? []) {
      runtimes[config.id] = emptyRuntime(config);
    }
    set({ activeProfileId: profile?.id ?? null, runtimes, selectedServiceId: null });
    void useSettingsStore.getState().set("devProfileId", profile?.id ?? null);
    if (profile) await get().reconcile();
  },

  selectService(serviceId) {
    set({ selectedServiceId: serviceId });
  },

  async saveProfile(profile) {
    await devApi.saveProfile(profile);
    const profiles = await devApi.listProfiles();
    set({ profiles });
    if (profile.id === get().activeProfileId) await get().selectProfile(profile.id);
  },

  async deleteProfile(profileId) {
    await devApi.deleteProfile(profileId);
    const profiles = await devApi.listProfiles();
    set({ profiles });
    if (get().activeProfileId === profileId) {
      await get().selectProfile(profiles[0]?.id ?? null);
    }
  },

  async saveService(config) {
    const { activeProfileId, profiles, runtimes } = get();
    const profile = profiles.find((p) => p.id === activeProfileId);
    if (!profile) return;

    const exists = profile.services.some((service) => service.id === config.id);
    const services = exists
      ? profile.services.map((service) => (service.id === config.id ? config : service))
      : [...profile.services, config];
    const updated = { ...profile, services };
    await devApi.saveProfile(updated);

    // ⚠️ Ne **pas** repasser par `selectProfile` : il reconstruit les runtimes
    // à neuf, ce qui effacerait les logs et l'état des services en marche.
    // Modifier une commande ne doit pas faire disparaître le service de l'écran.
    const previous = runtimes[config.id];
    set({
      profiles: profiles.map((p) => (p.id === updated.id ? updated : p)),
      runtimes: {
        ...runtimes,
        [config.id]: previous ? { ...previous, config } : emptyRuntime(config),
      },
    });
  },

  async removeService(serviceId) {
    const { activeProfileId, profiles, runtimes, selectedServiceId } = get();
    const profile = profiles.find((p) => p.id === activeProfileId);
    if (!profile) return;

    const updated = {
      ...profile,
      services: profile.services.filter((service) => service.id !== serviceId),
    };
    await devApi.saveProfile(updated);

    const nextRuntimes = { ...runtimes };
    delete nextRuntimes[serviceId];
    set({
      profiles: profiles.map((p) => (p.id === updated.id ? updated : p)),
      runtimes: nextRuntimes,
      selectedServiceId: selectedServiceId === serviceId ? null : selectedServiceId,
    });
  },

  async start(serviceId) {
    const profileId = get().activeProfileId;
    if (profileId) await devApi.start(profileId, serviceId);
  },

  async stop(serviceId) {
    const profileId = get().activeProfileId;
    if (profileId) await devApi.stop(profileId, serviceId);
  },

  async restart(serviceId) {
    const profileId = get().activeProfileId;
    if (profileId) await devApi.restart(profileId, serviceId);
  },

  async build(serviceId) {
    const profileId = get().activeProfileId;
    if (profileId) await devApi.build(profileId, serviceId);
  },

  async startMany(serviceIds) {
    const profileId = get().activeProfileId;
    if (profileId) await devApi.startBatch(profileId, serviceIds);
  },

  async stopMany(serviceIds) {
    const profileId = get().activeProfileId;
    if (profileId) await devApi.stopBatch(profileId, serviceIds);
  },

  async restartMany(serviceIds) {
    const profileId = get().activeProfileId;
    if (profileId) await devApi.restartBatch(profileId, serviceIds);
  },

  async scan(rootPath) {
    set({ scanning: true, scanProgress: null });
    try {
      return await devApi.scan(rootPath);
    } finally {
      set({ scanning: false, scanProgress: null });
    }
  },

  async reconcile() {
    const { activeProfileId, runtimes } = get();
    if (!activeProfileId) return;

    try {
      const [managed, probed] = await Promise.all([
        devApi.processList(activeProfileId),
        devApi.probe(activeProfileId),
      ]);

      const next = { ...runtimes };
      for (const info of managed) {
        const runtime = next[info.serviceId];
        if (!runtime) continue;
        next[info.serviceId] = {
          ...runtime,
          status: "running",
          pid: info.pid,
          startedAt: info.startedAt,
        };
      }
      for (const result of probed) {
        const runtime = next[result.serviceId];
        // Un service que nous gérons a déjà son état : ne pas l'écraser.
        if (!runtime || managed.some((m) => m.serviceId === result.serviceId)) continue;
        next[result.serviceId] = {
          ...runtime,
          // « externe » = il tourne, mais pas sous notre supervision : on ne
          // peut pas lui promettre un arrêt propre ni ses logs.
          status: result.detected ? "external" : "stopped",
        };
      }
      set({ runtimes: next });
    } catch (error) {
      console.error("dev: reconciliation", error);
    }
  },

  clearLogs(serviceId) {
    const runtime = get().runtimes[serviceId];
    if (!runtime) return;
    set({ runtimes: { ...get().runtimes, [serviceId]: { ...runtime, logs: [] } } });
  },

  subscribe() {
    const appendLog = (serviceId: string, entry: LogEntry) => {
      const runtime = get().runtimes[serviceId];
      if (!runtime) return;
      // Le backend regroupe déjà les lignes par paquets de 100 ms ; on les
      // éclate ici pour que la recherche et la coloration travaillent ligne
      // par ligne (lot 2.2).
      const lines = entry.text.split("\n").map((text) => ({ ...entry, text }));
      const logs = [...runtime.logs, ...lines];
      set({
        runtimes: {
          ...get().runtimes,
          [serviceId]: {
            ...runtime,
            logs: logs.length > MAX_LOG_LINES ? logs.slice(-MAX_LOG_LINES) : logs,
          },
        },
      });
    };

    const applyStatus = (update: StatusUpdate) => {
      const runtime = get().runtimes[update.serviceId];
      if (!runtime) return;
      set({
        runtimes: {
          ...get().runtimes,
          [update.serviceId]: {
            ...runtime,
            status: update.status,
            pid: update.pid,
            error: update.error,
            exitReason: update.exitReason,
            exitCode: update.exitCode,
            retryCount: update.retryCount,
            stuck: update.stuck,
            waitingFor: update.waitingFor,
            startedAt: update.status === "running" ? (runtime.startedAt ?? Date.now()) : undefined,
          },
        },
      });
    };

    const unsubscribers = [
      onServiceLog((event) => appendLog(event.serviceId, event.entry)),
      onServiceStatus(applyStatus),
      onScanProgress((progress) => set({ scanProgress: progress })),
    ];

    return () => {
      for (const unsubscribe of unsubscribers) unsubscribe();
    };
  },
}));
