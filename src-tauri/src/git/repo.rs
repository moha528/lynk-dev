//! Opérations `git`, traduites de `lynk-dev-electron/electron/git-handlers.ts`.
//!
//! On passe par le **binaire `git`**, pas par libgit2 : c'est ce que faisait la
//! version Electron (`execFile('git', args)`), donc les sorties, les
//! `credential.helper`, les hooks et le `.gitconfig` de l'utilisateur se
//! comportent exactement pareil.

use std::path::Path;
use std::time::Duration;

use anyhow::Result;

use crate::process::{self, DEFAULT_TIMEOUT};

use super::parse;
use super::types::{
    BranchListResult, ConflictSide, GitStatusResult, LogEntry, MergeOutcome, PushOutcome,
    RemoteInfo, RepoConfigResult, StashEntry,
};

/// Les opérations réseau (fetch/pull/push) méritent bien plus que les 30 s des
/// commandes locales : un `fetch --all` sur douze dépôts peut être long.
const NETWORK_TIMEOUT: Duration = Duration::from_secs(120);

/// Lance `git` et rend sa sortie ; une commande en échec devient une erreur.
async fn git(repo: &Path, args: &[&str]) -> Result<String> {
    process::run(repo, "git", args, DEFAULT_TIMEOUT).await
}

async fn git_net(repo: &Path, args: &[&str]) -> Result<String> {
    process::run(repo, "git", args, NETWORK_TIMEOUT).await
}

/// Variante tolérante : rend une chaîne vide plutôt qu'une erreur.
///
/// Indispensable pour l'interrogation d'un dépôt : un dépôt sans commit n'a pas
/// de `HEAD`, un dépôt sans distant n'a pas d'`@{u}`. Ce ne sont pas des pannes.
async fn git_safe(repo: &Path, args: &[&str]) -> String {
    git(repo, args).await.unwrap_or_default()
}

fn as_str_slice(values: &[String]) -> Vec<&str> {
    values.iter().map(String::as_str).collect()
}

// ── Interrogation ────────────────────────────────────────────────────────

pub async fn status(repo: &Path) -> Result<GitStatusResult> {
    // `core.quotePath=false` évite que `git` échappe les chemins accentués en
    // séquences octales — sans quoi `données/été.txt` arrive illisible.
    let raw = git_safe(
        repo,
        &["-c", "core.quotePath=false", "status", "--porcelain"],
    )
    .await;
    let parts = parse::parse_status(&raw);

    let branch = git_safe(repo, &["rev-parse", "--abbrev-ref", "HEAD"]).await;

    let mut ahead = 0;
    let mut behind = 0;
    let tracking = git_safe(
        repo,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )
    .await;
    if !tracking.is_empty() {
        let range = format!("{tracking}...HEAD");
        let raw = git_safe(repo, &["rev-list", "--left-right", "--count", &range]).await;
        (ahead, behind) = parse::parse_ahead_behind(&raw);
    }

    Ok(GitStatusResult {
        branch,
        ahead,
        behind,
        staged: parts.staged,
        modified: parts.modified,
        untracked: parts.untracked,
        conflicts: parts.conflicts,
    })
}

pub async fn branches(repo: &Path) -> Result<BranchListResult> {
    let current = git_safe(repo, &["rev-parse", "--abbrev-ref", "HEAD"]).await;
    let local =
        parse::parse_branch_list(&git_safe(repo, &["branch", "--format=%(refname:short)"]).await);
    let remote = parse::parse_remote_branch_list(
        &git_safe(repo, &["branch", "-r", "--format=%(refname:short)"]).await,
    );
    Ok(BranchListResult {
        current,
        local,
        remote,
    })
}

pub async fn log(repo: &Path, count: u32) -> Result<Vec<LogEntry>> {
    let count = format!("-{count}");
    let raw = git_safe(repo, &["log", &count, "--format=%H||%h||%s||%an||%ci||%D"]).await;
    Ok(parse::parse_log(&raw))
}

pub async fn stash_list(repo: &Path) -> Result<Vec<StashEntry>> {
    let raw = git_safe(repo, &["stash", "list", "--format=%gd||%gs||%ci"]).await;
    Ok(parse::parse_stash_list(&raw))
}

// ── Branches ─────────────────────────────────────────────────────────────

pub async fn checkout(repo: &Path, branch: &str) -> Result<()> {
    git(repo, &["checkout", branch]).await?;
    Ok(())
}

pub async fn create_branch(repo: &Path, name: &str, start_point: Option<&str>) -> Result<()> {
    let mut args = vec!["checkout", "-b", name];
    if let Some(start) = start_point {
        args.push(start);
    }
    git(repo, &args).await?;
    Ok(())
}

pub async fn delete_branch(repo: &Path, branch: &str, force: bool) -> Result<()> {
    git(repo, &["branch", if force { "-D" } else { "-d" }, branch]).await?;
    Ok(())
}

// ── Réseau ───────────────────────────────────────────────────────────────

pub async fn fetch(repo: &Path) -> Result<()> {
    git_net(repo, &["fetch", "--all", "--prune"]).await?;
    Ok(())
}

/// `pull`, avec les conflits rendus comme un **résultat** et non une erreur.
pub async fn pull(repo: &Path, branch: Option<&str>) -> Result<MergeOutcome> {
    let mut args = vec!["pull"];
    if let Some(branch) = branch {
        args.push("origin");
        args.push(branch);
    }
    match git_net(repo, &args).await {
        Ok(message) => Ok(MergeOutcome {
            success: true,
            conflicts: Vec::new(),
            message,
        }),
        Err(err) => conflict_outcome_or_error(repo, err).await,
    }
}

pub async fn push(repo: &Path, branch: Option<&str>, set_upstream: bool) -> Result<PushOutcome> {
    let mut args = vec!["push"];
    if set_upstream {
        args.push("-u");
    }
    if let Some(branch) = branch {
        args.push("origin");
        args.push(branch);
    }
    // Un `push` refusé (non fast-forward, droits manquants) est une information
    // à afficher, pas une exception : l'écran groupé doit pouvoir continuer sur
    // les autres dépôts.
    Ok(match git_net(repo, &args).await {
        Ok(message) => PushOutcome {
            success: true,
            message,
        },
        Err(err) => PushOutcome {
            success: false,
            message: format!("{err:#}"),
        },
    })
}

// ── Index ────────────────────────────────────────────────────────────────

pub async fn stage(repo: &Path, files: &[String]) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }
    // ⚠️ Le `--` n'est pas décoratif : sans lui, un fichier nommé `-f` ou
    // `--chmod=…` est lu par `git` comme une option. Les trois autres
    // fonctions d'index l'avaient, celle-ci l'avait perdu.
    let mut args = vec!["add".to_string(), "--".to_string()];
    args.extend_from_slice(files);
    git(repo, &as_str_slice(&args)).await?;
    Ok(())
}

pub async fn unstage(repo: &Path, files: &[String]) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }
    let mut args = vec!["reset".to_string(), "HEAD".to_string(), "--".to_string()];
    args.extend_from_slice(files);
    git(repo, &as_str_slice(&args)).await?;
    Ok(())
}

pub async fn stage_all(repo: &Path) -> Result<()> {
    git(repo, &["add", "-A"]).await?;
    Ok(())
}

/// Abandonne les modifications de l'arbre de travail.
///
/// ⚠️ `include_untracked` bascule sur `git clean -f`, qui **supprime** des
/// fichiers non suivis. Ce n'est pas la même opération que `checkout --`.
pub async fn discard_changes(repo: &Path, files: &[String], include_untracked: bool) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }
    let head = if include_untracked {
        vec!["clean".to_string(), "-f".to_string(), "--".to_string()]
    } else {
        vec!["checkout".to_string(), "--".to_string()]
    };
    let mut args = head;
    args.extend_from_slice(files);
    git(repo, &as_str_slice(&args)).await?;
    Ok(())
}

/// Ramène index **et** arbre de travail sur `HEAD`.
pub async fn discard_staged(repo: &Path, files: &[String]) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }
    let mut args = vec!["checkout".to_string(), "HEAD".to_string(), "--".to_string()];
    args.extend_from_slice(files);
    git(repo, &as_str_slice(&args)).await?;
    Ok(())
}

pub async fn commit(repo: &Path, message: &str, staged_files: &[String]) -> Result<()> {
    // Re-stage explicite : l'écran peut avoir coché des fichiers depuis le
    // dernier rafraîchissement, l'index doit refléter ce qu'on voit.
    if !staged_files.is_empty() {
        stage(repo, staged_files).await?;
    }
    git(repo, &["commit", "-m", message]).await?;
    Ok(())
}

// ── Contenu ──────────────────────────────────────────────────────────────

pub async fn diff(repo: &Path, file: &str, staged: bool) -> Result<String> {
    Ok(if staged {
        git_safe(repo, &["diff", "--cached", "--", file]).await
    } else {
        git_safe(repo, &["diff", "--", file]).await
    })
}

/// Diff complet de **ce qui est indexé**, tous fichiers confondus.
///
/// C'est l'entrée de la rédaction assistée d'un message de commit : l'index
/// fait foi, puisque c'est exactement ce qui sera validé.
pub async fn staged_diff(repo: &Path) -> Result<String> {
    Ok(git_safe(repo, &["diff", "--cached"]).await)
}

/// Contenu du fichier tel qu'il est dans `HEAD`.
pub async fn show_file(repo: &Path, file: &str) -> Result<String> {
    let spec = format!("HEAD:{file}");
    Ok(git_safe(repo, &["show", &spec]).await)
}

/// Contenu du fichier tel qu'il est sur le disque.
///
/// ⚠️ Le chemin est **contraint au dépôt**. `Path::join` a un piège : joindre un
/// chemin absolu **remplace** la base au lieu de s'y ajouter, donc un
/// `file_content(repo, "C:/Users/…/id_rsa")` lirait cette clé. Aucun appelant
/// légitime n'envoie ça — les chemins viennent de `git status`, qui les rend
/// relatifs — mais une commande qui peut lire n'importe quel fichier du disque
/// n'a pas à exister quand rien n'en a besoin.
pub async fn file_content(repo: &Path, file: &str) -> Result<String> {
    let Some(path) = within(repo, file) else {
        anyhow::bail!("chemin hors du dépôt : {file}");
    };
    Ok(tokio::fs::read_to_string(path).await.unwrap_or_default())
}

/// `repo/file`, ou `None` si `file` sort du dépôt (absolu, ou remontant par
/// `..`). Purement lexical : pas d'accès disque, donc pas de course entre la
/// vérification et la lecture.
fn within(repo: &Path, file: &str) -> Option<std::path::PathBuf> {
    use std::path::Component;

    let candidate = Path::new(file);
    if candidate.is_absolute() {
        return None;
    }
    for component in candidate.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            // `..`, une racine, ou un préfixe de lecteur Windows (`C:`).
            _ => return None,
        }
    }
    Some(repo.join(candidate))
}

// ── Fusion ───────────────────────────────────────────────────────────────

pub async fn merge(repo: &Path, branch: &str) -> Result<MergeOutcome> {
    match git(repo, &["merge", branch]).await {
        Ok(message) => Ok(MergeOutcome {
            success: true,
            conflicts: Vec::new(),
            message,
        }),
        Err(err) => conflict_outcome_or_error(repo, err).await,
    }
}

/// Un échec de fusion est soit un conflit (état légitime à présenter), soit une
/// vraie panne (dépôt absent, index verrouillé) qu'il faut remonter.
async fn conflict_outcome_or_error(repo: &Path, err: anyhow::Error) -> Result<MergeOutcome> {
    let message = format!("{err:#}");
    if !parse::is_merge_conflict(&message) {
        return Err(err);
    }
    let raw = git_safe(
        repo,
        &["-c", "core.quotePath=false", "status", "--porcelain"],
    )
    .await;
    Ok(MergeOutcome {
        success: false,
        conflicts: parse::parse_status(&raw).conflicts,
        message,
    })
}

pub async fn merge_abort(repo: &Path) -> Result<()> {
    git(repo, &["merge", "--abort"]).await?;
    Ok(())
}

/// Résout un conflit en gardant un seul des deux côtés, puis marque le fichier
/// comme résolu.
pub async fn resolve_conflict(repo: &Path, file: &str, side: ConflictSide) -> Result<()> {
    git(repo, &["checkout", side.flag(), "--", file]).await?;
    git(repo, &["add", "--", file]).await?;
    Ok(())
}

// ── Remisage ─────────────────────────────────────────────────────────────

pub async fn stash_save(repo: &Path, message: Option<&str>) -> Result<()> {
    let mut args = vec!["stash", "push"];
    if let Some(message) = message {
        args.push("-m");
        args.push(message);
    }
    git(repo, &args).await?;
    Ok(())
}

pub async fn stash_pop(repo: &Path, index: Option<u32>) -> Result<()> {
    let reference = index.map(|i| format!("stash@{{{i}}}"));
    let mut args = vec!["stash", "pop"];
    if let Some(reference) = &reference {
        args.push(reference);
    }
    git(repo, &args).await?;
    Ok(())
}

pub async fn stash_drop(repo: &Path, index: u32) -> Result<()> {
    let reference = format!("stash@{{{index}}}");
    git(repo, &["stash", "drop", &reference]).await?;
    Ok(())
}

// ── Configuration du dépôt ───────────────────────────────────────────────

pub async fn repo_config(repo: &Path) -> Result<RepoConfigResult> {
    let names = parse::parse_branch_list(&git_safe(repo, &["remote"]).await);
    let mut remotes = Vec::with_capacity(names.len());
    for name in names {
        let fetch_url = git_safe(repo, &["remote", "get-url", &name]).await;
        let push_url = git_safe(repo, &["remote", "get-url", "--push", &name]).await;
        let push_url = if push_url.is_empty() {
            fetch_url.clone()
        } else {
            push_url
        };
        remotes.push(RemoteInfo {
            name,
            fetch_url,
            push_url,
        });
    }

    let branches = parse::parse_branch_tracking(
        &git_safe(
            repo,
            &[
                "for-each-ref",
                "--format=%(refname:short)||%(upstream:short)||%(upstream:remotename)||%(upstream:remoteref:short)||%(upstream:track)",
                "refs/heads/",
            ],
        )
        .await,
    );

    let optional = |value: String| if value.is_empty() { None } else { Some(value) };

    Ok(RepoConfigResult {
        remotes,
        branches,
        user_name: optional(git_safe(repo, &["config", "--local", "user.name"]).await),
        user_email: optional(git_safe(repo, &["config", "--local", "user.email"]).await),
        global_user_name: optional(git_safe(repo, &["config", "--global", "user.name"]).await),
        global_user_email: optional(git_safe(repo, &["config", "--global", "user.email"]).await),
        default_branch: optional(git_safe(repo, &["config", "init.defaultBranch"]).await),
        is_bare: git_safe(repo, &["rev-parse", "--is-bare-repository"]).await == "true",
        worktree: git_safe(repo, &["rev-parse", "--show-toplevel"]).await,
        git_dir: git_safe(repo, &["rev-parse", "--git-dir"]).await,
    })
}

pub async fn set_config(repo: &Path, key: &str, value: &str, global: bool) -> Result<()> {
    let scope = if global { "--global" } else { "--local" };
    git(repo, &["config", scope, key, value]).await?;
    Ok(())
}

/// Retirer une clé absente n'est pas une erreur : `git` sort en code 5, ce que
/// l'appelant n'a aucune raison de voir remonter.
pub async fn unset_config(repo: &Path, key: &str, global: bool) -> Result<()> {
    let scope = if global { "--global" } else { "--local" };
    let _ = git(repo, &["config", scope, "--unset", key]).await;
    Ok(())
}

pub async fn add_remote(repo: &Path, name: &str, url: &str) -> Result<()> {
    git(repo, &["remote", "add", name, url]).await?;
    Ok(())
}

pub async fn remove_remote(repo: &Path, name: &str) -> Result<()> {
    git(repo, &["remote", "remove", name]).await?;
    Ok(())
}

pub async fn set_remote_url(repo: &Path, name: &str, url: &str, push: bool) -> Result<()> {
    let mut args = vec!["remote", "set-url"];
    if push {
        args.push("--push");
    }
    args.push(name);
    args.push(url);
    git(repo, &args).await?;
    Ok(())
}

pub async fn rename_remote(repo: &Path, old_name: &str, new_name: &str) -> Result<()> {
    git(repo, &["remote", "rename", old_name, new_name]).await?;
    Ok(())
}

pub async fn set_branch_upstream(repo: &Path, branch: &str, upstream: &str) -> Result<()> {
    let flag = format!("--set-upstream-to={upstream}");
    git(repo, &["branch", &flag, branch]).await?;
    Ok(())
}

pub async fn unset_branch_upstream(repo: &Path, branch: &str) -> Result<()> {
    git(repo, &["branch", "--unset-upstream", branch]).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Crée un dépôt jetable avec un commit, pour éprouver le chemin réel.
    async fn fixture() -> (tempfile::TempDir, std::path::PathBuf) {
        let tmp = tempfile::tempdir().expect("tmp");
        let repo = tmp.path().to_path_buf();
        git(&repo, &["init", "-q"]).await.expect("init");
        git(&repo, &["config", "user.email", "test@lynk.dev"])
            .await
            .expect("email");
        git(&repo, &["config", "user.name", "Test"])
            .await
            .expect("name");
        // Certaines configurations globales signent les commits : on désactive
        // pour que le test ne dépende pas de la machine.
        git(&repo, &["config", "commit.gpgsign", "false"])
            .await
            .expect("gpgsign");
        // `core.autocrlf=true` est le défaut d'installation sous Windows : un
        // `checkout` y réécrit les LF en CRLF, et le contenu restauré ne serait
        // plus octet pour octet celui du commit. Ce n'est pas un défaut du port
        // — c'est `git` qui fait ce qu'on lui a demandé — mais ça rendrait le
        // test dépendant du poste.
        git(&repo, &["config", "core.autocrlf", "false"])
            .await
            .expect("autocrlf");
        tokio::fs::write(repo.join("a.txt"), "un\n")
            .await
            .expect("write");
        stage_all(&repo).await.expect("add");
        commit(&repo, "init", &[]).await.expect("commit");
        (tmp, repo)
    }

    #[tokio::test]
    async fn status_sees_a_new_file_then_its_staging() {
        let (_tmp, repo) = fixture().await;
        tokio::fs::write(repo.join("b.txt"), "deux\n")
            .await
            .expect("write");

        let state = status(&repo).await.expect("status");
        assert_eq!(state.untracked, vec!["b.txt"]);
        assert!(state.staged.is_empty());

        stage(&repo, &["b.txt".to_string()]).await.expect("stage");
        let state = status(&repo).await.expect("status");
        assert!(state.untracked.is_empty());
        assert_eq!(state.staged.len(), 1);
        assert_eq!(state.staged[0].path, "b.txt");
    }

    #[tokio::test]
    async fn unstage_puts_the_file_back_as_untracked() {
        let (_tmp, repo) = fixture().await;
        tokio::fs::write(repo.join("b.txt"), "deux\n")
            .await
            .expect("write");
        stage(&repo, &["b.txt".to_string()]).await.expect("stage");
        unstage(&repo, &["b.txt".to_string()])
            .await
            .expect("unstage");

        let state = status(&repo).await.expect("status");
        assert_eq!(state.untracked, vec!["b.txt"]);
    }

    #[tokio::test]
    async fn commit_then_log_reports_the_message() {
        let (_tmp, repo) = fixture().await;
        tokio::fs::write(repo.join("a.txt"), "modifie\n")
            .await
            .expect("write");
        commit(&repo, "feat: modifie a", &["a.txt".to_string()])
            .await
            .expect("commit");

        let entries = log(&repo, 5).await.expect("log");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message, "feat: modifie a");
        assert!(!entries[0].hash.is_empty());
    }

    #[tokio::test]
    async fn branches_lists_the_new_branch_and_follows_checkout() {
        let (_tmp, repo) = fixture().await;
        create_branch(&repo, "feature/x", None)
            .await
            .expect("branch");

        let list = branches(&repo).await.expect("branches");
        assert_eq!(list.current, "feature/x");
        assert!(list.local.contains(&"feature/x".to_string()));
        assert!(list.remote.is_empty(), "aucun distant sur un depot local");
    }

    #[tokio::test]
    async fn discard_changes_restores_the_file() {
        let (_tmp, repo) = fixture().await;
        tokio::fs::write(repo.join("a.txt"), "casse\n")
            .await
            .expect("write");
        discard_changes(&repo, &["a.txt".to_string()], false)
            .await
            .expect("discard");
        let content = tokio::fs::read_to_string(repo.join("a.txt"))
            .await
            .expect("read");
        assert_eq!(content, "un\n");
    }

    #[tokio::test]
    async fn stash_round_trip() {
        let (_tmp, repo) = fixture().await;
        tokio::fs::write(repo.join("a.txt"), "en cours\n")
            .await
            .expect("write");

        stash_save(&repo, Some("mon remisage")).await.expect("save");
        let stashes = stash_list(&repo).await.expect("list");
        assert_eq!(stashes.len(), 1);
        assert_eq!(stashes[0].index, 0);
        assert!(stashes[0].message.contains("mon remisage"));

        stash_pop(&repo, Some(0)).await.expect("pop");
        assert!(stash_list(&repo).await.expect("list").is_empty());
    }

    #[tokio::test]
    async fn repo_config_reports_local_identity_and_no_remote() {
        let (_tmp, repo) = fixture().await;
        let config = repo_config(&repo).await.expect("config");
        assert_eq!(config.user_email.as_deref(), Some("test@lynk.dev"));
        assert!(config.remotes.is_empty());
        assert!(!config.is_bare);
        assert!(!config.worktree.is_empty());
    }

    #[tokio::test]
    async fn remotes_can_be_added_renamed_and_removed() {
        let (_tmp, repo) = fixture().await;
        add_remote(&repo, "origin", "https://example.invalid/x.git")
            .await
            .expect("add");
        assert_eq!(repo_config(&repo).await.expect("config").remotes.len(), 1);

        rename_remote(&repo, "origin", "upstream")
            .await
            .expect("rename");
        let config = repo_config(&repo).await.expect("config");
        assert_eq!(config.remotes[0].name, "upstream");

        remove_remote(&repo, "upstream").await.expect("remove");
        assert!(repo_config(&repo).await.expect("config").remotes.is_empty());
    }

    #[tokio::test]
    async fn diff_and_show_file_read_both_versions() {
        let (_tmp, repo) = fixture().await;
        tokio::fs::write(repo.join("a.txt"), "deux\n")
            .await
            .expect("write");

        let unstaged = diff(&repo, "a.txt", false).await.expect("diff");
        assert!(unstaged.contains("-un"), "le diff doit montrer l'ancien");
        assert!(unstaged.contains("+deux"));

        let head = show_file(&repo, "a.txt").await.expect("show");
        assert_eq!(head, "un");

        let disk = file_content(&repo, "a.txt").await.expect("content");
        assert_eq!(disk, "deux\n");
    }

    /// Un fichier absent ne doit pas faire échouer la lecture.
    #[tokio::test]
    async fn file_content_is_empty_for_a_missing_file() {
        let (_tmp, repo) = fixture().await;
        assert_eq!(file_content(&repo, "absent.txt").await.expect("ok"), "");
    }

    #[tokio::test]
    async fn unset_config_on_a_missing_key_is_not_an_error() {
        let (_tmp, repo) = fixture().await;
        unset_config(&repo, "lynk.inexistant", false)
            .await
            .expect("ne doit pas echouer");
    }

    /// Le cas qui compte : une fusion en conflit rend un résultat exploitable,
    /// pas une erreur.
    #[tokio::test]
    async fn merge_conflict_is_returned_as_an_outcome() {
        let (_tmp, repo) = fixture().await;
        let base = branches(&repo).await.expect("branches").current;

        create_branch(&repo, "autre", None).await.expect("branch");
        tokio::fs::write(repo.join("a.txt"), "version autre\n")
            .await
            .expect("write");
        commit(&repo, "autre", &["a.txt".to_string()])
            .await
            .expect("commit");

        checkout(&repo, &base).await.expect("checkout");
        tokio::fs::write(repo.join("a.txt"), "version base\n")
            .await
            .expect("write");
        commit(&repo, "base", &["a.txt".to_string()])
            .await
            .expect("commit");

        let outcome = merge(&repo, "autre").await.expect("pas une erreur");
        assert!(!outcome.success);
        assert_eq!(outcome.conflicts.len(), 1);
        assert_eq!(outcome.conflicts[0].path, "a.txt");

        // Et on doit pouvoir en sortir.
        resolve_conflict(&repo, "a.txt", ConflictSide::Ours)
            .await
            .expect("resolve");
        let state = status(&repo).await.expect("status");
        assert!(state.conflicts.is_empty(), "le conflit est resolu");
    }

    #[tokio::test]
    async fn merge_abort_restores_the_branch() {
        let (_tmp, repo) = fixture().await;
        let base = branches(&repo).await.expect("branches").current;

        create_branch(&repo, "autre", None).await.expect("branch");
        tokio::fs::write(repo.join("a.txt"), "autre\n")
            .await
            .expect("write");
        commit(&repo, "autre", &["a.txt".to_string()])
            .await
            .expect("commit");

        checkout(&repo, &base).await.expect("checkout");
        tokio::fs::write(repo.join("a.txt"), "base\n")
            .await
            .expect("write");
        commit(&repo, "base", &["a.txt".to_string()])
            .await
            .expect("commit");

        let outcome = merge(&repo, "autre").await.expect("outcome");
        assert!(!outcome.success);
        merge_abort(&repo).await.expect("abort");
        assert!(status(&repo).await.expect("status").conflicts.is_empty());
    }

    /// Un `push` sans distant échoue **proprement** : l'écran groupé doit
    /// pouvoir enchaîner sur les autres dépôts.
    #[tokio::test]
    async fn push_without_a_remote_fails_without_raising() {
        let (_tmp, repo) = fixture().await;
        let outcome = push(&repo, None, false).await.expect("pas une erreur");
        assert!(!outcome.success);
        assert!(!outcome.message.is_empty());
    }

    /// Le piège de `Path::join` : un chemin **absolu** remplace la base au lieu
    /// de s'y ajouter. Sans garde, la lecture sortirait du dépôt.
    #[test]
    fn an_absolute_or_climbing_path_is_refused() {
        let repo = Path::new("/depot");
        assert!(within(repo, "src/main.rs").is_some());
        assert!(within(repo, "./src/main.rs").is_some());
        // Un nom accentué ou avec des espaces reste légitime.
        assert!(within(repo, "données/été 2026.txt").is_some());

        assert!(within(repo, "../secret").is_none());
        assert!(within(repo, "src/../../secret").is_none());
        assert!(within(repo, "/etc/passwd").is_none());
        if cfg!(windows) {
            assert!(within(repo, "C:/Users/PC/.ssh/id_rsa").is_none());
            assert!(within(repo, r"\\serveur\partage").is_none());
        }
    }

    /// Un fichier dont le nom commence par `-` doit pouvoir être indexé : sans
    /// le `--`, `git add` le lit comme une option et refuse.
    #[tokio::test]
    async fn a_file_named_like_an_option_can_be_staged() {
        let (_tmp, repo) = fixture().await;
        tokio::fs::write(repo.join("--suspect.txt"), "contenu\n")
            .await
            .expect("write");

        stage(&repo, &["--suspect.txt".to_string()])
            .await
            .expect("l'indexation ne doit pas prendre le nom pour une option");

        let staged = status(&repo).await.expect("status").staged;
        assert!(
            staged.iter().any(|entry| entry.path == "--suspect.txt"),
            "attendu dans l'index, obtenu {staged:?}"
        );
    }
}
