import { invoke } from "@tauri-apps/api/core";

import type {
  BranchListResult,
  ConflictSide,
  GitProfile,
  GitStatusResult,
  LogEntry,
  MergeOutcome,
  PushOutcome,
  RepoConfigResult,
  RepoScanResult,
  StashEntry,
} from "./types";

/**
 * Surface IPC du Git Manager.
 *
 * Un seul endroit connaît les noms de commandes ; les composants et le store
 * passent par ici, jamais par `invoke` en direct.
 */
export const gitApi = {
  // ── Profils ──────────────────────────────────────────────────────────
  listProfiles: () => invoke<GitProfile[]>("git_profile_list"),
  saveProfile: (profile: GitProfile) => invoke<void>("git_profile_save", { profile }),
  deleteProfile: (profileId: string) => invoke<void>("git_profile_delete", { profileId }),
  scanRepos: (rootPath: string) => invoke<RepoScanResult[]>("git_scan_repos", { rootPath }),

  // ── Interrogation ────────────────────────────────────────────────────
  status: (repoPath: string) => invoke<GitStatusResult>("git_status", { repoPath }),
  branches: (repoPath: string) => invoke<BranchListResult>("git_branches", { repoPath }),
  log: (repoPath: string, count?: number) => invoke<LogEntry[]>("git_log", { repoPath, count }),
  stashList: (repoPath: string) => invoke<StashEntry[]>("git_stash_list", { repoPath }),
  repoConfig: (repoPath: string) => invoke<RepoConfigResult>("git_repo_config", { repoPath }),

  // ── Branches ─────────────────────────────────────────────────────────
  checkout: (repoPath: string, branch: string) =>
    invoke<void>("git_checkout", { repoPath, branch }),
  createBranch: (repoPath: string, name: string, startPoint?: string) =>
    invoke<void>("git_create_branch", { repoPath, name, startPoint }),
  deleteBranch: (repoPath: string, branch: string, force: boolean) =>
    invoke<void>("git_delete_branch", { repoPath, branch, force }),

  // ── Réseau ───────────────────────────────────────────────────────────
  fetch: (repoPath: string) => invoke<void>("git_fetch", { repoPath }),
  pull: (repoPath: string, branch?: string) =>
    invoke<MergeOutcome>("git_pull", { repoPath, branch }),
  push: (repoPath: string, branch?: string, setUpstream?: boolean) =>
    invoke<PushOutcome>("git_push", { repoPath, branch, setUpstream }),

  // ── Index ────────────────────────────────────────────────────────────
  stage: (repoPath: string, files: string[]) => invoke<void>("git_stage", { repoPath, files }),
  unstage: (repoPath: string, files: string[]) => invoke<void>("git_unstage", { repoPath, files }),
  stageAll: (repoPath: string) => invoke<void>("git_stage_all", { repoPath }),
  discardChanges: (repoPath: string, files: string[], includeUntracked?: boolean) =>
    invoke<void>("git_discard_changes", { repoPath, files, includeUntracked }),
  discardStaged: (repoPath: string, files: string[]) =>
    invoke<void>("git_discard_staged", { repoPath, files }),
  commit: (repoPath: string, message: string, stagedFiles?: string[]) =>
    invoke<void>("git_commit", { repoPath, message, stagedFiles }),

  // ── Contenu ──────────────────────────────────────────────────────────
  diff: (repoPath: string, filePath: string, staged: boolean) =>
    invoke<string>("git_diff", { repoPath, filePath, staged }),
  showFile: (repoPath: string, filePath: string) =>
    invoke<string>("git_show_file", { repoPath, filePath }),
  fileContent: (repoPath: string, filePath: string) =>
    invoke<string>("git_file_content", { repoPath, filePath }),

  // ── Fusion ───────────────────────────────────────────────────────────
  merge: (repoPath: string, branch: string) =>
    invoke<MergeOutcome>("git_merge", { repoPath, branch }),
  mergeAbort: (repoPath: string) => invoke<void>("git_merge_abort", { repoPath }),
  resolveConflict: (repoPath: string, filePath: string, side: ConflictSide) =>
    invoke<void>("git_resolve_conflict", { repoPath, filePath, side }),

  // ── Remisage ─────────────────────────────────────────────────────────
  stashSave: (repoPath: string, message?: string) =>
    invoke<void>("git_stash_save", { repoPath, message }),
  stashPop: (repoPath: string, index?: number) =>
    invoke<void>("git_stash_pop", { repoPath, index }),
  stashDrop: (repoPath: string, index: number) =>
    invoke<void>("git_stash_drop", { repoPath, index }),

  // ── Configuration ────────────────────────────────────────────────────
  setConfig: (repoPath: string, key: string, value: string, global?: boolean) =>
    invoke<void>("git_set_config", { repoPath, key, value, global }),
  unsetConfig: (repoPath: string, key: string, global?: boolean) =>
    invoke<void>("git_unset_config", { repoPath, key, global }),
  addRemote: (repoPath: string, name: string, url: string) =>
    invoke<void>("git_add_remote", { repoPath, name, url }),
  removeRemote: (repoPath: string, name: string) =>
    invoke<void>("git_remove_remote", { repoPath, name }),
  setRemoteUrl: (repoPath: string, name: string, url: string, push?: boolean) =>
    invoke<void>("git_set_remote_url", { repoPath, name, url, push }),
  renameRemote: (repoPath: string, oldName: string, newName: string) =>
    invoke<void>("git_rename_remote", { repoPath, oldName, newName }),
  setBranchUpstream: (repoPath: string, branch: string, upstream: string) =>
    invoke<void>("git_set_branch_upstream", { repoPath, branch, upstream }),
  unsetBranchUpstream: (repoPath: string, branch: string) =>
    invoke<void>("git_unset_branch_upstream", { repoPath, branch }),

  // ── Shell ────────────────────────────────────────────────────────────
  openInTerminal: (dirPath: string) => invoke<void>("git_open_in_terminal", { dirPath }),
};
