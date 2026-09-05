import { create } from "zustand";

import { useSettingsStore } from "@/stores/useSettingsStore";

import { gitApi } from "./ipc";
import type { BatchOutcome, ConflictSide, GitProfile, RepoState } from "./types";

/**
 * Nombre de dépôts traités en parallèle.
 *
 * Chaque opération lance un process `git` : sur un profil de douze dépôts, tout
 * lancer d'un coup sature la machine et rend la fenêtre saccadée. Quatre suffit
 * à masquer la latence sans faire ramer le poste.
 */
const CONCURRENCY = 4;

/** Nombre de commits ramenés par dépôt — au-delà, on ne lit plus. */
const LOG_COUNT = 30;

async function mapLimit<T, R>(
  items: T[],
  limit: number,
  worker: (item: T) => Promise<R>,
): Promise<R[]> {
  const results: R[] = new Array(items.length);
  let cursor = 0;
  const runners = Array.from({ length: Math.min(limit, items.length) }, async () => {
    for (;;) {
      const index = cursor++;
      if (index >= items.length) return;
      results[index] = await worker(items[index]);
    }
  });
  await Promise.all(runners);
  return results;
}

function emptyRepo(path: string): RepoState {
  return {
    path,
    name: path.split(/[\\/]/).filter(Boolean).pop() ?? path,
    status: null,
    branches: null,
    log: [],
    stashes: [],
    config: null,
    loading: false,
    error: null,
  };
}

type GitState = {
  profiles: GitProfile[];
  activeProfileId: string | null;
  /** Indexé par chemin absolu du dépôt. */
  repos: Record<string, RepoState>;
  selectedRepoPath: string | null;
  loading: boolean;
  /** Une opération groupée est en cours. */
  busy: boolean;

  hydrate: () => Promise<void>;
  selectProfile: (profileId: string | null) => Promise<void>;
  selectRepo: (repoPath: string | null) => void;
  saveProfile: (profile: GitProfile) => Promise<void>;
  deleteProfile: (profileId: string) => Promise<void>;

  refreshRepo: (repoPath: string) => Promise<void>;
  refreshAll: () => Promise<void>;
  loadConfig: (repoPath: string) => Promise<void>;

  stage: (repoPath: string, files: string[]) => Promise<void>;
  unstage: (repoPath: string, files: string[]) => Promise<void>;
  stageAll: (repoPath: string) => Promise<void>;
  discard: (repoPath: string, files: string[], untracked: boolean) => Promise<void>;
  commit: (repoPath: string, message: string) => Promise<void>;

  checkout: (repoPath: string, branch: string) => Promise<void>;
  createBranch: (repoPath: string, name: string, startPoint?: string) => Promise<void>;
  deleteBranch: (repoPath: string, branch: string, force: boolean) => Promise<void>;
  merge: (repoPath: string, branch: string) => Promise<void>;
  mergeAbort: (repoPath: string) => Promise<void>;
  resolveConflict: (repoPath: string, filePath: string, side: ConflictSide) => Promise<void>;

  stashSave: (repoPath: string, message?: string) => Promise<void>;
  stashPop: (repoPath: string, index: number) => Promise<void>;
  stashDrop: (repoPath: string, index: number) => Promise<void>;

  /** Opérations réseau groupées ; rendent un compte rendu par dépôt. */
  fetchMany: (repoPaths: string[]) => Promise<BatchOutcome[]>;
  pullMany: (repoPaths: string[]) => Promise<BatchOutcome[]>;
  pushMany: (repoPaths: string[]) => Promise<BatchOutcome[]>;
};

export const useGitStore = create<GitState>((set, get) => {
  const patch = (repoPath: string, changes: Partial<RepoState>) => {
    const current = get().repos[repoPath];
    if (!current) return;
    set({ repos: { ...get().repos, [repoPath]: { ...current, ...changes } } });
  };

  /** Toute mutation est suivie d'un rafraîchissement : l'état vient de `git`,
   *  jamais d'une supposition de l'écran. */
  const mutate = async (repoPath: string, action: () => Promise<void>) => {
    await action();
    await get().refreshRepo(repoPath);
  };

  return {
    profiles: [],
    activeProfileId: null,
    repos: {},
    selectedRepoPath: null,
    loading: false,
    busy: false,

    async hydrate() {
      set({ loading: true });
      try {
        const profiles = await gitApi.listProfiles();
        const remembered = useSettingsStore.getState().gitProfileId;
        const known = profiles.some((p) => p.id === remembered) ? remembered : null;
        set({ profiles, loading: false });
        await get().selectProfile(get().activeProfileId ?? known ?? profiles[0]?.id ?? null);
      } catch (error) {
        console.error("git: chargement des profils", error);
        set({ loading: false });
      }
    },

    async selectProfile(profileId) {
      const profile = get().profiles.find((p) => p.id === profileId) ?? null;
      const repos: Record<string, RepoState> = {};
      for (const path of profile?.repoPaths ?? []) repos[path] = emptyRepo(path);

      set({
        activeProfileId: profile?.id ?? null,
        repos,
        selectedRepoPath: profile?.repoPaths[0] ?? null,
      });
      void useSettingsStore.getState().set("gitProfileId", profile?.id ?? null);
      if (profile) await get().refreshAll();
    },

    selectRepo(repoPath) {
      set({ selectedRepoPath: repoPath });
    },

    async saveProfile(profile) {
      await gitApi.saveProfile(profile);
      set({ profiles: await gitApi.listProfiles() });
      if (profile.id === get().activeProfileId) await get().selectProfile(profile.id);
    },

    async deleteProfile(profileId) {
      await gitApi.deleteProfile(profileId);
      const profiles = await gitApi.listProfiles();
      set({ profiles });
      if (get().activeProfileId === profileId) {
        await get().selectProfile(profiles[0]?.id ?? null);
      }
    },

    async refreshRepo(repoPath) {
      patch(repoPath, { loading: true, error: null });
      try {
        const [status, branches, log, stashes] = await Promise.all([
          gitApi.status(repoPath),
          gitApi.branches(repoPath),
          gitApi.log(repoPath, LOG_COUNT),
          gitApi.stashList(repoPath),
        ]);
        patch(repoPath, { status, branches, log, stashes, loading: false });
      } catch (error) {
        patch(repoPath, {
          loading: false,
          error: error instanceof Error ? error.message : String(error),
        });
      }
    },

    async refreshAll() {
      const paths = Object.keys(get().repos);
      await mapLimit(paths, CONCURRENCY, (path) => get().refreshRepo(path));
    },

    async loadConfig(repoPath) {
      try {
        patch(repoPath, { config: await gitApi.repoConfig(repoPath) });
      } catch (error) {
        patch(repoPath, { error: error instanceof Error ? error.message : String(error) });
      }
    },

    stage: (repoPath, files) => mutate(repoPath, () => gitApi.stage(repoPath, files)),
    unstage: (repoPath, files) => mutate(repoPath, () => gitApi.unstage(repoPath, files)),
    stageAll: (repoPath) => mutate(repoPath, () => gitApi.stageAll(repoPath)),
    discard: (repoPath, files, untracked) =>
      mutate(repoPath, () => gitApi.discardChanges(repoPath, files, untracked)),

    commit: (repoPath, message) =>
      mutate(repoPath, async () => {
        // Les fichiers déjà en index sont la source de vérité : on ne renvoie
        // pas la sélection de l'écran, qui pourrait être périmée.
        await gitApi.commit(repoPath, message);
      }),

    checkout: (repoPath, branch) => mutate(repoPath, () => gitApi.checkout(repoPath, branch)),
    createBranch: (repoPath, name, startPoint) =>
      mutate(repoPath, () => gitApi.createBranch(repoPath, name, startPoint)),
    deleteBranch: (repoPath, branch, force) =>
      mutate(repoPath, () => gitApi.deleteBranch(repoPath, branch, force)),

    merge: (repoPath, branch) =>
      mutate(repoPath, async () => {
        const outcome = await gitApi.merge(repoPath, branch);
        // Un conflit n'est pas une erreur : le rafraîchissement qui suit le
        // fera apparaître dans l'onglet Modifications.
        if (!outcome.success && outcome.conflicts.length === 0) {
          throw new Error(outcome.message);
        }
      }),
    mergeAbort: (repoPath) => mutate(repoPath, () => gitApi.mergeAbort(repoPath)),
    resolveConflict: (repoPath, filePath, side) =>
      mutate(repoPath, () => gitApi.resolveConflict(repoPath, filePath, side)),

    stashSave: (repoPath, message) => mutate(repoPath, () => gitApi.stashSave(repoPath, message)),
    stashPop: (repoPath, index) => mutate(repoPath, () => gitApi.stashPop(repoPath, index)),
    stashDrop: (repoPath, index) => mutate(repoPath, () => gitApi.stashDrop(repoPath, index)),

    async fetchMany(repoPaths) {
      return runBatch(repoPaths, async (repoPath) => {
        await gitApi.fetch(repoPath);
        return { success: true, message: "à jour" };
      });
    },

    async pullMany(repoPaths) {
      return runBatch(repoPaths, async (repoPath) => {
        const outcome = await gitApi.pull(repoPath);
        return {
          success: outcome.success,
          message: outcome.success ? "à jour" : outcome.message,
          conflicts: outcome.conflicts,
        };
      });
    },

    async pushMany(repoPaths) {
      return runBatch(repoPaths, async (repoPath) => {
        const outcome = await gitApi.push(repoPath);
        return { success: outcome.success, message: outcome.message };
      });
    },
  };

  /**
   * Exécute une opération réseau sur plusieurs dépôts.
   *
   * ⚠️ Un dépôt en échec **ne doit pas** interrompre les autres : sur douze
   * dépôts, un seul distant injoignable annulerait tout le reste.
   */
  async function runBatch(
    repoPaths: string[],
    worker: (repoPath: string) => Promise<Omit<BatchOutcome, "repoPath" | "repoName">>,
  ): Promise<BatchOutcome[]> {
    set({ busy: true });
    try {
      const outcomes = await mapLimit(repoPaths, CONCURRENCY, async (repoPath) => {
        const name = get().repos[repoPath]?.name ?? repoPath;
        try {
          const result = await worker(repoPath);
          return { repoPath, repoName: name, ...result };
        } catch (error) {
          return {
            repoPath,
            repoName: name,
            success: false,
            message: error instanceof Error ? error.message : String(error),
          };
        }
      });
      await get().refreshAll();
      return outcomes;
    } finally {
      set({ busy: false });
    }
  }
});
