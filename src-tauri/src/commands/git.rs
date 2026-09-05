//! Pont IPC du Git Manager.
//!
//! Comme pour le Dev Manager : **aucune logique métier ici**. Chaque commande
//! convertit ses arguments et délègue à `crate::git`.

use std::path::{Path, PathBuf};

use tauri::State;

use crate::git::types::{
    BranchListResult, ConflictSide, GitProfile, GitStatusResult, LogEntry, MergeOutcome,
    PushOutcome, RepoConfigResult, RepoScanResult, StashEntry,
};
use crate::git::{repo, scan, shell};
use crate::store::{git_profiles as dao, DbPool};
use crate::AppError;

fn at(path: &str) -> PathBuf {
    Path::new(path).to_path_buf()
}

// ── Profils ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn git_profile_list(pool: State<'_, DbPool>) -> Result<Vec<GitProfile>, AppError> {
    Ok(dao::all(pool.inner()).await?)
}

#[tauri::command]
pub async fn git_profile_save(
    pool: State<'_, DbPool>,
    profile: GitProfile,
) -> Result<(), AppError> {
    dao::save(pool.inner(), &profile).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_profile_delete(
    pool: State<'_, DbPool>,
    profile_id: String,
) -> Result<(), AppError> {
    dao::delete(pool.inner(), &profile_id).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_scan_repos(root_path: String) -> Result<Vec<RepoScanResult>, AppError> {
    Ok(scan::scan_repos(&at(&root_path)).await)
}

// ── Interrogation ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn git_status(repo_path: String) -> Result<GitStatusResult, AppError> {
    Ok(repo::status(&at(&repo_path)).await?)
}

#[tauri::command]
pub async fn git_branches(repo_path: String) -> Result<BranchListResult, AppError> {
    Ok(repo::branches(&at(&repo_path)).await?)
}

#[tauri::command]
pub async fn git_log(repo_path: String, count: Option<u32>) -> Result<Vec<LogEntry>, AppError> {
    Ok(repo::log(&at(&repo_path), count.unwrap_or(20)).await?)
}

#[tauri::command]
pub async fn git_stash_list(repo_path: String) -> Result<Vec<StashEntry>, AppError> {
    Ok(repo::stash_list(&at(&repo_path)).await?)
}

#[tauri::command]
pub async fn git_repo_config(repo_path: String) -> Result<RepoConfigResult, AppError> {
    Ok(repo::repo_config(&at(&repo_path)).await?)
}

// ── Branches ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn git_checkout(repo_path: String, branch: String) -> Result<(), AppError> {
    repo::checkout(&at(&repo_path), &branch).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_create_branch(
    repo_path: String,
    name: String,
    start_point: Option<String>,
) -> Result<(), AppError> {
    repo::create_branch(&at(&repo_path), &name, start_point.as_deref()).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_delete_branch(
    repo_path: String,
    branch: String,
    force: bool,
) -> Result<(), AppError> {
    repo::delete_branch(&at(&repo_path), &branch, force).await?;
    Ok(())
}

// ── Réseau ───────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn git_fetch(repo_path: String) -> Result<(), AppError> {
    repo::fetch(&at(&repo_path)).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_pull(repo_path: String, branch: Option<String>) -> Result<MergeOutcome, AppError> {
    Ok(repo::pull(&at(&repo_path), branch.as_deref()).await?)
}

#[tauri::command]
pub async fn git_push(
    repo_path: String,
    branch: Option<String>,
    set_upstream: Option<bool>,
) -> Result<PushOutcome, AppError> {
    Ok(repo::push(
        &at(&repo_path),
        branch.as_deref(),
        set_upstream.unwrap_or(false),
    )
    .await?)
}

// ── Index ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn git_stage(repo_path: String, files: Vec<String>) -> Result<(), AppError> {
    repo::stage(&at(&repo_path), &files).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_unstage(repo_path: String, files: Vec<String>) -> Result<(), AppError> {
    repo::unstage(&at(&repo_path), &files).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_stage_all(repo_path: String) -> Result<(), AppError> {
    repo::stage_all(&at(&repo_path)).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_discard_changes(
    repo_path: String,
    files: Vec<String>,
    include_untracked: Option<bool>,
) -> Result<(), AppError> {
    repo::discard_changes(&at(&repo_path), &files, include_untracked.unwrap_or(false)).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_discard_staged(repo_path: String, files: Vec<String>) -> Result<(), AppError> {
    repo::discard_staged(&at(&repo_path), &files).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_commit(
    repo_path: String,
    message: String,
    staged_files: Option<Vec<String>>,
) -> Result<(), AppError> {
    repo::commit(&at(&repo_path), &message, &staged_files.unwrap_or_default()).await?;
    Ok(())
}

// ── Contenu ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn git_diff(
    repo_path: String,
    file_path: String,
    staged: bool,
) -> Result<String, AppError> {
    Ok(repo::diff(&at(&repo_path), &file_path, staged).await?)
}

#[tauri::command]
pub async fn git_show_file(repo_path: String, file_path: String) -> Result<String, AppError> {
    Ok(repo::show_file(&at(&repo_path), &file_path).await?)
}

#[tauri::command]
pub async fn git_file_content(repo_path: String, file_path: String) -> Result<String, AppError> {
    Ok(repo::file_content(&at(&repo_path), &file_path).await?)
}

// ── Fusion ───────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn git_merge(repo_path: String, branch: String) -> Result<MergeOutcome, AppError> {
    Ok(repo::merge(&at(&repo_path), &branch).await?)
}

#[tauri::command]
pub async fn git_merge_abort(repo_path: String) -> Result<(), AppError> {
    repo::merge_abort(&at(&repo_path)).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_resolve_conflict(
    repo_path: String,
    file_path: String,
    side: ConflictSide,
) -> Result<(), AppError> {
    repo::resolve_conflict(&at(&repo_path), &file_path, side).await?;
    Ok(())
}

// ── Remisage ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn git_stash_save(repo_path: String, message: Option<String>) -> Result<(), AppError> {
    repo::stash_save(&at(&repo_path), message.as_deref()).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_stash_pop(repo_path: String, index: Option<u32>) -> Result<(), AppError> {
    repo::stash_pop(&at(&repo_path), index).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_stash_drop(repo_path: String, index: u32) -> Result<(), AppError> {
    repo::stash_drop(&at(&repo_path), index).await?;
    Ok(())
}

// ── Configuration ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn git_set_config(
    repo_path: String,
    key: String,
    value: String,
    global: Option<bool>,
) -> Result<(), AppError> {
    repo::set_config(&at(&repo_path), &key, &value, global.unwrap_or(false)).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_unset_config(
    repo_path: String,
    key: String,
    global: Option<bool>,
) -> Result<(), AppError> {
    repo::unset_config(&at(&repo_path), &key, global.unwrap_or(false)).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_add_remote(repo_path: String, name: String, url: String) -> Result<(), AppError> {
    repo::add_remote(&at(&repo_path), &name, &url).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_remove_remote(repo_path: String, name: String) -> Result<(), AppError> {
    repo::remove_remote(&at(&repo_path), &name).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_set_remote_url(
    repo_path: String,
    name: String,
    url: String,
    push: Option<bool>,
) -> Result<(), AppError> {
    repo::set_remote_url(&at(&repo_path), &name, &url, push.unwrap_or(false)).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_rename_remote(
    repo_path: String,
    old_name: String,
    new_name: String,
) -> Result<(), AppError> {
    repo::rename_remote(&at(&repo_path), &old_name, &new_name).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_set_branch_upstream(
    repo_path: String,
    branch: String,
    upstream: String,
) -> Result<(), AppError> {
    repo::set_branch_upstream(&at(&repo_path), &branch, &upstream).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_unset_branch_upstream(repo_path: String, branch: String) -> Result<(), AppError> {
    repo::unset_branch_upstream(&at(&repo_path), &branch).await?;
    Ok(())
}

// ── Shell ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn git_open_in_terminal(dir_path: String) -> Result<(), AppError> {
    shell::open_in_terminal(&at(&dir_path)).await?;
    Ok(())
}
