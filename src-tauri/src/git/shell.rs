//! Ouverture d'un terminal dans un dossier.
//!
//! Traduction de `lynk-dev-electron/electron/git-handlers.ts:20-49` et `:702`.
//!
//! Une implémentation par famille d'OS plutôt qu'une fonction truffée de `cfg` :
//! les trois chemins n'ont rien en commun, et les mélanger produisait des
//! imports inutilisés selon la plateforme compilée.
//!
//! L'ouverture de l'explorateur de fichiers, elle, passe par le plugin `opener`
//! côté front (`revealItemInDir`) : rien à faire ici.

use std::path::Path;
use std::process::Stdio;

use anyhow::Result;
use tokio::process::Command;

/// Ouvre un terminal dans `dir`. Le process est détaché : il survit à Lynk Dev.
#[cfg(windows)]
pub async fn open_in_terminal(dir: &Path) -> Result<()> {
    // `start` détache la console, `/K` la garde ouverte après le `cd` — c'est
    // tout l'intérêt de l'action.
    Command::new("cmd.exe")
        .args(["/C", "start", "cmd.exe", "/K", "cd", "/d"])
        .arg(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

#[cfg(target_os = "macos")]
pub async fn open_in_terminal(dir: &Path) -> Result<()> {
    Command::new("open")
        .arg("-a")
        .arg("Terminal")
        .arg(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

/// Terminaux Linux essayés **dans cet ordre**.
///
/// L'ordre n'est pas arbitraire : d'abord ceux qui correspondent aux
/// environnements de bureau courants, puis les terminaux modernes autonomes, et
/// `xterm` en dernier recours. `x-terminal-emulator` ferme la marche : c'est un
/// lien Debian qui n'existe nulle part ailleurs.
///
/// Un préfixe qui finit par `=` se colle au chemin ; sinon le chemin est un
/// argument distinct.
#[cfg(all(unix, not(target_os = "macos")))]
const LINUX_TERMINALS: &[(&str, &str)] = &[
    ("gnome-terminal", "--working-directory="),
    ("konsole", "--workdir "),
    ("xfce4-terminal", "--working-directory="),
    ("mate-terminal", "--working-directory="),
    ("lxterminal", "--working-directory "),
    ("tilix", "-w "),
    ("terminator", "--working-directory "),
    ("alacritty", "--working-directory "),
    ("kitty", "--directory "),
    ("wezterm", "start --cwd "),
    ("urxvt", "-cd "),
    ("xterm", "-e sh -c cd "),
    ("x-terminal-emulator", "--working-directory="),
];

#[cfg(all(unix, not(target_os = "macos")))]
pub async fn open_in_terminal(dir: &Path) -> Result<()> {
    let path = dir.to_string_lossy().to_string();

    for (program, prefix) in LINUX_TERMINALS {
        if !command_exists(program).await {
            continue;
        }

        let mut args: Vec<String> = prefix.split_whitespace().map(str::to_string).collect();
        if prefix.ends_with('=') {
            let last = args.pop().unwrap_or_default();
            args.push(format!("{last}{path}"));
        } else {
            args.push(path.clone());
        }

        let spawned = Command::new(program)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        if spawned.is_ok() {
            return Ok(());
        }
    }

    anyhow::bail!(
        "aucun terminal trouvé — essayés : {}",
        LINUX_TERMINALS
            .iter()
            .map(|(program, _)| *program)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// `command -v` est intégré à tout shell POSIX : contrairement à `which`, il est
/// toujours présent, y compris sur une image minimale.
#[cfg(all(unix, not(target_os = "macos")))]
async fn command_exists(program: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {program} >/dev/null 2>&1"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|status| status.success())
        .unwrap_or(false)
}
