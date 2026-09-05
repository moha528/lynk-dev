//! Analyseurs des sorties de `git`.
//!
//! Traduction de `lynk-dev-electron/electron/git-handlers.ts:101-160` et des
//! parseurs disséminés dans les handlers.
//!
//! Tout est **pur** : aucune de ces fonctions ne lance `git`. C'est ce qui rend
//! la partie la plus fragile du port testable sans dépôt de test, et ce qui
//! garantit qu'on reproduit exactement le découpage d'origine.

use super::types::{
    BranchTracking, ConflictFile, FileChange, FileStatus, LogEntry, StashEntry, StatusParts,
};

/// Séparateur des formats `--format=...` : improbable dans un message de commit.
pub const SEP: &str = "||";

/// Analyse une sortie `git status --porcelain` (version 1).
///
/// Rappel du format : deux colonnes de code (index puis arbre de travail), une
/// espace, puis le chemin. Un renommage s'écrit `R  ancien -> nouveau`.
pub fn parse_status(raw: &str) -> StatusParts {
    let mut parts = StatusParts::default();

    for line in raw.lines().filter(|line| !line.is_empty()) {
        let mut chars = line.chars();
        let (Some(x), Some(y)) = (chars.next(), chars.next()) else {
            continue;
        };
        // `line[3..]` en octets casserait sur un chemin accentué : on saute
        // trois *caractères* (les deux codes plus l'espace).
        let path: String = line.chars().skip(3).collect();
        if path.is_empty() {
            continue;
        }

        // Conflits : l'un des deux côtés est « unmerged », ou les deux ont
        // ajouté / supprimé le même chemin.
        if x == 'U' || y == 'U' || (x == 'A' && y == 'A') || (x == 'D' && y == 'D') {
            parts.conflicts.push(ConflictFile {
                path,
                ours_status: x.to_string(),
                theirs_status: y.to_string(),
            });
            continue;
        }

        // Index (« staged »).
        if x != ' ' && x != '?' {
            let mut change = FileChange {
                path: path.clone(),
                status: FileStatus::from_code(x),
                old_path: None,
            };
            if x == 'R' || x == 'C' {
                if let Some((old, new)) = path.split_once(" -> ") {
                    change.old_path = Some(old.to_string());
                    change.path = new.to_string();
                }
            }
            parts.staged.push(change);
        }

        // Arbre de travail.
        if y == 'M' || y == 'D' {
            parts.modified.push(FileChange {
                path: path.clone(),
                status: FileStatus::from_code(y),
                old_path: None,
            });
        }

        if x == '?' && y == '?' {
            parts.untracked.push(path);
        }
    }

    parts
}

/// Analyse `git rev-list --left-right --count <upstream>...HEAD`.
///
/// La sortie est `derrière<TAB>devant` — **dans cet ordre**. L'inverser est
/// l'erreur classique, et elle est invisible tant que les deux nombres sont
/// égaux.
pub fn parse_ahead_behind(raw: &str) -> (u32, u32) {
    let mut fields = raw.split('\t');
    let behind = fields
        .next()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);
    let ahead = fields
        .next()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);
    (ahead, behind)
}

/// Liste de branches issue de `git branch --format=%(refname:short)`.
pub fn parse_branch_list(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// Idem pour les branches distantes, en écartant les pointeurs `.../HEAD` qui
/// ne sont pas des branches mais des alias.
pub fn parse_remote_branch_list(raw: &str) -> Vec<String> {
    parse_branch_list(raw)
        .into_iter()
        .filter(|branch| !branch.ends_with("/HEAD"))
        .collect()
}

/// Analyse `git stash list --format=%gd||%gs||%ci`.
pub fn parse_stash_list(raw: &str) -> Vec<StashEntry> {
    raw.lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let mut fields = line.split(SEP);
            let reference = fields.next()?;
            let message = fields.next().unwrap_or_default().to_string();
            let date = fields.next().unwrap_or_default().to_string();
            let index = reference
                .trim_start_matches("stash@{")
                .trim_end_matches('}')
                .parse()
                .ok()?;
            Some(StashEntry {
                index,
                message,
                date,
            })
        })
        .collect()
}

/// Analyse `git log --format=%H||%h||%s||%an||%ci||%D`.
pub fn parse_log(raw: &str) -> Vec<LogEntry> {
    raw.lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let mut fields = line.split(SEP);
            Some(LogEntry {
                hash: fields.next()?.to_string(),
                short_hash: fields.next().unwrap_or_default().to_string(),
                message: fields.next().unwrap_or_default().to_string(),
                author: fields.next().unwrap_or_default().to_string(),
                date: fields.next().unwrap_or_default().to_string(),
                refs: fields.next().unwrap_or_default().to_string(),
            })
        })
        .collect()
}

/// Analyse le `for-each-ref` du suivi de branches :
/// `%(refname:short)||%(upstream:short)||%(upstream:remotename)||%(upstream:remoteref:short)||%(upstream:track)`
pub fn parse_branch_tracking(raw: &str) -> Vec<BranchTracking> {
    fn non_empty(value: Option<&str>) -> Option<String> {
        value.filter(|v| !v.is_empty()).map(str::to_string)
    }

    raw.lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let mut fields = line.split(SEP);
            let local = fields.next()?.to_string();
            if local.is_empty() {
                return None;
            }
            let remote = non_empty(fields.next());
            let remote_name = non_empty(fields.next());
            let remote_branch = non_empty(fields.next());
            let track = fields.next().unwrap_or_default();
            Some(BranchTracking {
                local,
                remote,
                remote_name,
                remote_branch,
                gone: track.contains("gone"),
            })
        })
        .collect()
}

/// Une sortie d'erreur `git` qui décrit un conflit de fusion plutôt qu'une
/// panne. Détermine si l'on rend un résultat « échec propre » ou une erreur.
pub fn is_merge_conflict(message: &str) -> bool {
    message.contains("CONFLICT") || message.contains("Automatic merge failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_splits_index_worktree_and_untracked() {
        let raw = "\
M  src/lib.rs
 M README.md
A  nouveau.txt
?? brouillon.md
D  supprime.rs
";
        let parts = parse_status(raw);
        assert_eq!(parts.staged.len(), 3, "M, A et D en index");
        assert_eq!(parts.staged[0].path, "src/lib.rs");
        assert_eq!(parts.staged[0].status, FileStatus::Modified);
        assert_eq!(parts.staged[2].status, FileStatus::Deleted);

        assert_eq!(parts.modified.len(), 1);
        assert_eq!(parts.modified[0].path, "README.md");

        assert_eq!(parts.untracked, vec!["brouillon.md"]);
        assert!(parts.conflicts.is_empty());
    }

    /// Un fichier modifié des deux côtés apparaît **deux fois** : une entrée
    /// en index, une dans l'arbre de travail. C'est voulu — ce sont deux
    /// versions différentes à présenter séparément.
    #[test]
    fn status_reports_a_file_staged_and_modified_twice() {
        let parts = parse_status("MM src/lib.rs\n");
        assert_eq!(parts.staged.len(), 1);
        assert_eq!(parts.modified.len(), 1);
    }

    #[test]
    fn status_extracts_rename_old_and_new_paths() {
        let parts = parse_status("R  ancien.rs -> nouveau.rs\n");
        assert_eq!(parts.staged.len(), 1);
        let change = &parts.staged[0];
        assert_eq!(change.status, FileStatus::Renamed);
        assert_eq!(change.old_path.as_deref(), Some("ancien.rs"));
        assert_eq!(change.path, "nouveau.rs");
    }

    #[test]
    fn status_detects_the_three_conflict_shapes() {
        let parts = parse_status(
            "UU les-deux.rs\nAA ajoute-par-les-deux.rs\nDD supprime-par-les-deux.rs\n",
        );
        assert_eq!(parts.conflicts.len(), 3);
        assert_eq!(parts.conflicts[0].path, "les-deux.rs");
        assert_eq!(parts.conflicts[0].ours_status, "U");
        // Un conflit n'apparaît jamais aussi comme « staged ».
        assert!(parts.staged.is_empty());
    }

    /// Le piège du découpage : un chemin accentué décale l'index si on coupe
    /// en octets au lieu de caractères.
    #[test]
    fn status_handles_accented_paths() {
        let parts = parse_status("?? données/été.txt\n");
        assert_eq!(parts.untracked, vec!["données/été.txt"]);
    }

    #[test]
    fn status_tolerates_empty_and_truncated_lines() {
        let parts = parse_status("");
        assert_eq!(parts, StatusParts::default());
        // Ligne trop courte : ignorée sans paniquer.
        let parts = parse_status("M\n\n?? ok.txt\n");
        assert_eq!(parts.untracked, vec!["ok.txt"]);
    }

    #[test]
    fn ahead_behind_reads_behind_first() {
        assert_eq!(parse_ahead_behind("3\t7"), (7, 3), "derriere=3, devant=7");
        assert_eq!(parse_ahead_behind("0\t0"), (0, 0));
        assert_eq!(parse_ahead_behind(""), (0, 0));
        assert_eq!(parse_ahead_behind("n'importe quoi"), (0, 0));
    }

    #[test]
    fn remote_branches_drop_the_head_pointer() {
        let raw = "origin/HEAD\norigin/main\norigin/feature/x\n";
        assert_eq!(
            parse_remote_branch_list(raw),
            vec!["origin/main", "origin/feature/x"]
        );
    }

    #[test]
    fn stash_list_reads_index_message_and_date() {
        let raw = "stash@{0}||WIP on main: 1234567 message||2026-09-05 10:00:00 +0000\n\
stash@{1}||On main: essai||2026-09-04 09:00:00 +0000\n";
        let stashes = parse_stash_list(raw);
        assert_eq!(stashes.len(), 2);
        assert_eq!(stashes[0].index, 0);
        assert_eq!(stashes[0].message, "WIP on main: 1234567 message");
        assert_eq!(stashes[1].index, 1);
    }

    #[test]
    fn log_reads_all_six_fields() {
        let raw = "abc123||abc||feat: ajoute X||Moha||2026-09-05 10:00:00 +0000||HEAD -> main, origin/main\n";
        let entries = parse_log(raw);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].hash, "abc123");
        assert_eq!(entries[0].short_hash, "abc");
        assert_eq!(entries[0].message, "feat: ajoute X");
        assert_eq!(entries[0].author, "Moha");
        assert_eq!(entries[0].refs, "HEAD -> main, origin/main");
    }

    /// Un commit sans référence laisse le dernier champ vide : la ligne doit
    /// quand même être lue.
    #[test]
    fn log_tolerates_a_missing_trailing_field() {
        let entries = parse_log("abc123||abc||msg||Moha||2026-09-05||\n");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].refs, "");
    }

    #[test]
    fn branch_tracking_reads_upstream_and_gone_flag() {
        let raw = "main||origin/main||origin||main||[ahead 2]\n\
orpheline||||||\n\
morte||origin/morte||origin||morte||[gone]\n";
        let branches = parse_branch_tracking(raw);
        assert_eq!(branches.len(), 3);

        assert_eq!(branches[0].local, "main");
        assert_eq!(branches[0].remote.as_deref(), Some("origin/main"));
        assert!(!branches[0].gone);

        assert_eq!(branches[1].local, "orpheline");
        assert_eq!(
            branches[1].remote, None,
            "pas d'upstream = None, pas une chaine vide"
        );

        assert!(branches[2].gone, "la branche distante a disparu");
    }

    #[test]
    fn merge_conflict_is_recognised_from_git_output() {
        assert!(is_merge_conflict(
            "CONFLICT (content): Merge conflict in a.rs"
        ));
        assert!(is_merge_conflict("Automatic merge failed; fix conflicts"));
        assert!(!is_merge_conflict("fatal: not a git repository"));
    }
}
