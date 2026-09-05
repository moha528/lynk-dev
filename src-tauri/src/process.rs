//! Exécution de binaires externes (`git`, `docker`, gestionnaires de paquets…).
//!
//! Équivalent Rust du helper `execFileAsync` de la version Electron
//! (`lynk-dev-electron/electron/git-handlers.ts:55-68`). Le port des modules est
//! une **traduction 1:1** : pour que les parseurs de sortie restent valables,
//! ce helper doit offrir exactement les mêmes garanties —
//!
//! - sortie normalisée en **LF**, sans saut de ligne final ;
//! - `stderr` remonté comme message d'erreur lisible quand le code est non nul ;
//! - **aucune fenêtre de console** sur Windows ;
//! - un **délai maximal**, au terme duquel le process est tué.

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use tokio::process::Command;

/// Délai par défaut, aligné sur celui de la version Electron (30 s).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Sortie complète d'un process terminé, code de retour compris.
#[derive(Debug, Clone)]
pub struct Output {
    /// `None` si le process a été interrompu par un signal.
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl Output {
    /// Le process s'est terminé avec le code 0.
    pub fn ok(&self) -> bool {
        self.code == Some(0)
    }
}

/// Normalise une sortie brute : UTF-8 tolérant, CRLF → LF, pas de saut final.
fn normalize(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw)
        .replace("\r\n", "\n")
        .trim_end_matches('\n')
        .to_string()
}

fn build(cwd: &Path, program: &str, args: &[&str]) -> Command {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Au dépassement du délai, la future est abandonnée : sans ceci, le
        // process resterait vivant en orphelin.
        .kill_on_drop(true);

    #[cfg(windows)]
    {
        // Équivalent de `windowsHide: true` côté Electron. Sans ce drapeau,
        // chaque appel fait clignoter une fenêtre de console — et un scan de
        // dépôts en fait des dizaines à la seconde.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd
}

/// Lance `program` et rend sa sortie **quel que soit le code de retour**.
///
/// À utiliser quand un code non nul est une information et non une erreur :
/// `git diff --quiet`, `git rev-parse --verify`, sondes de santé…
pub async fn run_raw(
    cwd: &Path,
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<Output> {
    let child = build(cwd, program, args)
        .spawn()
        .with_context(|| format!("lancement de `{program}` dans {}", cwd.display()))?;

    let out = tokio::time::timeout(timeout, child.wait_with_output())
        .await
        .with_context(|| format!("`{program}` n'a pas rendu la main en {timeout:?}"))?
        .with_context(|| format!("attente de `{program}`"))?;

    Ok(Output {
        code: out.status.code(),
        stdout: normalize(&out.stdout),
        stderr: normalize(&out.stderr),
    })
}

/// Lance `program` et rend son `stdout` normalisé.
///
/// Un code de retour non nul devient une erreur portant le `stderr` du process
/// (ou son `stdout` s'il est muet), comme le faisait la version Electron.
pub async fn run(cwd: &Path, program: &str, args: &[&str], timeout: Duration) -> Result<String> {
    let out = run_raw(cwd, program, args, timeout).await?;
    if out.ok() {
        return Ok(out.stdout);
    }
    let detail = if !out.stderr.is_empty() {
        out.stderr
    } else if !out.stdout.is_empty() {
        out.stdout
    } else {
        format!("code de sortie {:?}", out.code)
    };
    bail!("{detail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn here() -> std::path::PathBuf {
        std::env::current_dir().expect("cwd")
    }

    #[test]
    fn normalize_converts_crlf_and_trims_trailing_newlines() {
        assert_eq!(normalize(b"a\r\nb\r\n"), "a\nb");
        assert_eq!(normalize(b"solo\n\n\n"), "solo");
        assert_eq!(normalize(b""), "");
    }

    /// `git` est présent partout où ce projet se construit (dev et CI).
    #[tokio::test]
    async fn run_returns_stdout() {
        let out = run(&here(), "git", &["--version"], DEFAULT_TIMEOUT)
            .await
            .expect("git --version");
        assert!(out.starts_with("git version"), "sortie inattendue: {out}");
        assert!(!out.ends_with('\n'), "le saut final doit etre retire");
    }

    /// Un code non nul doit devenir une erreur portant le detail du process.
    #[tokio::test]
    async fn run_turns_nonzero_exit_into_an_error() {
        let err = run(
            &here(),
            "git",
            &[
                "rev-parse",
                "--verify",
                "refs/heads/definitely-not-a-branch",
            ],
            DEFAULT_TIMEOUT,
        )
        .await
        .expect_err("cette reference ne doit pas exister");
        assert!(!format!("{err:#}").is_empty());
    }

    /// Le meme appel via `run_raw` rend le code au lieu d'echouer.
    #[tokio::test]
    async fn run_raw_reports_the_exit_code() {
        let out = run_raw(
            &here(),
            "git",
            &[
                "rev-parse",
                "--verify",
                "refs/heads/definitely-not-a-branch",
            ],
            DEFAULT_TIMEOUT,
        )
        .await
        .expect("le lancement lui-meme doit reussir");
        assert!(!out.ok());
    }

    #[tokio::test]
    async fn run_fails_when_the_program_does_not_exist() {
        let err = run(&here(), "lynk-no-such-binary", &[], DEFAULT_TIMEOUT)
            .await
            .expect_err("binaire inexistant");
        assert!(format!("{err:#}").contains("lynk-no-such-binary"));
    }
}
