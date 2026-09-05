//! Recherche de dépôts Git sous une racine.
//!
//! Traduction de `lynk-dev-electron/electron/git-handlers.ts:261-288`.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use super::types::RepoScanResult;

/// Profondeur maximale explorée sous la racine.
const MAX_DEPTH: usize = 2;

pub async fn is_git_repo(dir: &Path) -> bool {
    // `.git` est un dossier dans un dépôt normal, un **fichier** dans un
    // worktree lié ou un sous-module : tester l'existence, pas le type.
    tokio::fs::metadata(dir.join(".git")).await.is_ok()
}

/// Liste les dépôts trouvés sous `root`.
///
/// ⚠️ On ne descend **jamais** dans un dépôt déjà trouvé : sinon un dépôt à
/// sous-modules remonterait chacun d'eux comme un dépôt de premier rang.
pub async fn scan_repos(root: &Path) -> Vec<RepoScanResult> {
    let mut found = Vec::new();
    let mut queue: VecDeque<(PathBuf, usize)> = VecDeque::new();
    queue.push_back((root.to_path_buf(), 0));

    while let Some((dir, depth)) = queue.pop_front() {
        if is_git_repo(&dir).await {
            found.push(RepoScanResult {
                name: dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| dir.to_string_lossy().to_string()),
                path: dir.to_string_lossy().to_string(),
            });
            continue;
        }

        if depth >= MAX_DEPTH {
            continue;
        }

        let Ok(mut entries) = tokio::fs::read_dir(&dir).await else {
            continue;
        };
        let mut children = Vec::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "node_modules" {
                continue;
            }
            // `file_type` ne suit pas les liens : un lien symbolique circulaire
            // ferait tourner le parcours indéfiniment.
            let Ok(file_type) = entry.file_type().await else {
                continue;
            };
            if file_type.is_dir() && !file_type.is_symlink() {
                children.push(entry.path());
            }
        }
        // Ordre stable d'une exécution à l'autre.
        children.sort();
        for child in children {
            queue.push_back((child, depth + 1));
        }
    }

    found.sort_by(|a, b| a.path.cmp(&b.path));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn make_repo(path: &Path) {
        tokio::fs::create_dir_all(path.join(".git"))
            .await
            .expect("mkdir");
    }

    #[tokio::test]
    async fn finds_repos_two_levels_down() {
        let tmp = tempfile::tempdir().expect("tmp");
        make_repo(&tmp.path().join("back").join("service-a")).await;
        make_repo(&tmp.path().join("back").join("service-b")).await;
        make_repo(&tmp.path().join("front")).await;

        let found = scan_repos(tmp.path()).await;
        let names: Vec<&str> = found.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"service-a"));
        assert!(names.contains(&"front"));
    }

    /// Le piège des sous-modules : un dépôt dans un dépôt ne doit pas remonter.
    #[tokio::test]
    async fn does_not_descend_into_a_repo() {
        let tmp = tempfile::tempdir().expect("tmp");
        let parent = tmp.path().join("parent");
        make_repo(&parent).await;
        make_repo(&parent.join("sous-module")).await;

        let found = scan_repos(tmp.path()).await;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "parent");
    }

    #[tokio::test]
    async fn skips_node_modules_and_hidden_directories() {
        let tmp = tempfile::tempdir().expect("tmp");
        make_repo(&tmp.path().join("node_modules").join("dep")).await;
        make_repo(&tmp.path().join(".cache").join("truc")).await;
        make_repo(&tmp.path().join("vrai")).await;

        let found = scan_repos(tmp.path()).await;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "vrai");
    }

    #[tokio::test]
    async fn a_root_that_is_itself_a_repo_is_returned() {
        let tmp = tempfile::tempdir().expect("tmp");
        make_repo(tmp.path()).await;
        assert_eq!(scan_repos(tmp.path()).await.len(), 1);
    }

    #[tokio::test]
    async fn an_empty_root_yields_nothing() {
        let tmp = tempfile::tempdir().expect("tmp");
        assert!(scan_repos(tmp.path()).await.is_empty());
    }
}
