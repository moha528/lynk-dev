/**
 * Contrat du Git Manager — miroir exact de `src-tauri/src/git/types.rs`.
 *
 * Comme pour le Dev Manager : le backend sérialise en camelCase, une divergence
 * ne casse pas la compilation mais produit des `undefined` silencieux.
 */

export type FileStatus = "added" | "modified" | "deleted" | "renamed" | "copied" | "untracked";

export interface FileChange {
  path: string;
  status: FileStatus;
  /** Renseigné pour un renommage ou une copie. */
  oldPath?: string;
}

export interface ConflictFile {
  path: string;
  /** Code d'index — « nous ». */
  oursStatus: string;
  /** Code d'arbre de travail — « eux ». */
  theirsStatus: string;
}

export interface GitStatusResult {
  branch: string;
  ahead: number;
  behind: number;
  staged: FileChange[];
  modified: FileChange[];
  untracked: string[];
  conflicts: ConflictFile[];
}

export interface BranchListResult {
  current: string;
  local: string[];
  remote: string[];
}

export interface StashEntry {
  index: number;
  message: string;
  date: string;
}

export interface LogEntry {
  hash: string;
  shortHash: string;
  message: string;
  author: string;
  date: string;
  refs: string;
}

/**
 * Résultat d'une fusion ou d'un `pull`.
 *
 * Un conflit n'est **pas** une erreur : c'est un état du dépôt à présenter.
 */
export interface MergeOutcome {
  success: boolean;
  conflicts: ConflictFile[];
  message: string;
}

export interface PushOutcome {
  success: boolean;
  message: string;
}

export interface RemoteInfo {
  name: string;
  fetchUrl: string;
  pushUrl: string;
}

export interface BranchTracking {
  local: string;
  remote: string | null;
  remoteName: string | null;
  remoteBranch: string | null;
  /** La branche distante a disparu. */
  gone: boolean;
}

export interface RepoConfigResult {
  remotes: RemoteInfo[];
  branches: BranchTracking[];
  userName: string | null;
  userEmail: string | null;
  globalUserName: string | null;
  globalUserEmail: string | null;
  defaultBranch: string | null;
  isBare: boolean;
  worktree: string;
  gitDir: string;
}

export interface RepoScanResult {
  path: string;
  name: string;
}

export interface GitProfile {
  id: string;
  name: string;
  rootPath: string;
  repoPaths: string[];
  createdAt: number;
}

export type ConflictSide = "ours" | "theirs";

/** État d'un dépôt tel que l'écran le manipule. */
export interface RepoState {
  path: string;
  name: string;
  status: GitStatusResult | null;
  branches: BranchListResult | null;
  log: LogEntry[];
  stashes: StashEntry[];
  config: RepoConfigResult | null;
  loading: boolean;
  error: string | null;
}

/** Résultat d'une opération groupée, dépôt par dépôt. */
export interface BatchOutcome {
  repoPath: string;
  repoName: string;
  success: boolean;
  message: string;
  conflicts?: ConflictFile[];
}

/** Nombre total de fichiers touchés — sert au badge de la liste. */
export function dirtyCount(status: GitStatusResult | null): number {
  if (!status) return 0;
  return (
    status.staged.length +
    status.modified.length +
    status.untracked.length +
    status.conflicts.length
  );
}
