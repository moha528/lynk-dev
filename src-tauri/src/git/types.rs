//! Types du Git Manager.
//!
//! Miroir de `lynk-dev-electron/packages/git-manager/src/types.ts`, sérialisé en
//! camelCase pour que le front porté n'ait rien à renommer.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Untracked,
}

impl FileStatus {
    /// Traduit un code de `git status --porcelain`. Tout code inconnu vaut
    /// « modifié » — comme côté Electron : mieux vaut afficher le fichier avec
    /// un état approximatif que le faire disparaître de la liste.
    pub fn from_code(code: char) -> Self {
        match code {
            'A' => Self::Added,
            'M' => Self::Modified,
            'D' => Self::Deleted,
            'R' => Self::Renamed,
            'C' => Self::Copied,
            '?' => Self::Untracked,
            _ => Self::Modified,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub status: FileStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictFile {
    pub path: String,
    /// Code d'index (« nous »).
    pub ours_status: String,
    /// Code d'arbre de travail (« eux »).
    pub theirs_status: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusParts {
    pub staged: Vec<FileChange>,
    pub modified: Vec<FileChange>,
    pub untracked: Vec<String>,
    pub conflicts: Vec<ConflictFile>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatusResult {
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub staged: Vec<FileChange>,
    pub modified: Vec<FileChange>,
    pub untracked: Vec<String>,
    pub conflicts: Vec<ConflictFile>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchListResult {
    pub current: String,
    pub local: Vec<String>,
    pub remote: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StashEntry {
    pub index: u32,
    pub message: String,
    pub date: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub date: String,
    pub refs: String,
}

/// Résultat d'une opération qui peut échouer *proprement* sur des conflits.
///
/// Un conflit n'est pas une erreur technique : c'est un état du dépôt que
/// l'écran doit présenter. D'où un résultat, et non une exception.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeOutcome {
    pub success: bool,
    pub conflicts: Vec<ConflictFile>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PushOutcome {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteInfo {
    pub name: String,
    pub fetch_url: String,
    pub push_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchTracking {
    pub local: String,
    /// Ex. `origin/main`.
    pub remote: Option<String>,
    /// Ex. `origin`.
    pub remote_name: Option<String>,
    /// Ex. `main`.
    pub remote_branch: Option<String>,
    /// La branche distante a disparu (`[gone]`).
    pub gone: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoConfigResult {
    pub remotes: Vec<RemoteInfo>,
    pub branches: Vec<BranchTracking>,
    pub user_name: Option<String>,
    pub user_email: Option<String>,
    pub global_user_name: Option<String>,
    pub global_user_email: Option<String>,
    pub default_branch: Option<String>,
    pub is_bare: bool,
    pub worktree: String,
    pub git_dir: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoScanResult {
    pub path: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitProfile {
    pub id: String,
    pub name: String,
    pub root_path: String,
    /// Chemins absolus des dépôts retenus dans ce profil.
    #[serde(default)]
    pub repo_paths: Vec<String>,
    /// Millisecondes depuis l'époque Unix.
    pub created_at: i64,
}

/// Stratégie de résolution d'un conflit, côté nous ou côté eux.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConflictSide {
    Ours,
    Theirs,
}

impl ConflictSide {
    pub fn flag(self) -> &'static str {
        match self {
            Self::Ours => "--ours",
            Self::Theirs => "--theirs",
        }
    }
}
