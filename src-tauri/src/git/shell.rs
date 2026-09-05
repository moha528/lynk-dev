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
///
/// ⚠️ **La ligne est construite à la main, guillemets compris.** `cmd.exe`
/// **re-analyse** ce qui suit `/C`, et y traite `&`, `|`, `^`, `<`, `>` comme
/// des séparateurs quand ils ne sont pas entre guillemets. Or Rust ne met des
/// guillemets que si l'argument contient un espace : un chemin tout à fait
/// ordinaire comme `C:\Dev\R&D\projet` était donc coupé au `&`, et la queue
/// (`D\projet`) exécutée comme une commande. Encadrer nous-mêmes règle les deux
/// problèmes à la fois — un chemin Windows ne peut pas contenir de guillemet,
/// il n'y a donc rien à échapper à l'intérieur.
///
/// Le `""` après `start` est le **titre de la fenêtre** : sans lui, `start`
/// prendrait le premier élément entre guillemets pour un titre au lieu d'un
/// programme.
#[cfg(windows)]
pub async fn open_in_terminal(dir: &Path) -> Result<()> {
    let mut command = Command::new("cmd.exe");
    command.raw_arg(format!(
        "/C start \"\" cmd.exe /K cd /d \"{}\"",
        dir.display()
    ));
    command
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
/// argument distinct. `None` = ce terminal n'a pas d'option de répertoire, on
/// s'en remet au **répertoire courant hérité**, qu'on positionne de toute façon.
///
/// ⚠️ `xterm` était listé avec `-e sh -c cd `, ce qui ne fait pas ce que ça
/// annonce : `sh -c cd <dir>` exécute le script `cd` **sans argument** (le
/// chemin devient `$0`), donc la fenêtre s'ouvrait sur le mauvais dossier et se
/// refermait aussitôt. `xterm` n'a pas d'option de répertoire de travail : la
/// seule voie correcte est l'héritage.
#[cfg(all(unix, not(target_os = "macos")))]
const LINUX_TERMINALS: &[(&str, Option<&str>)] = &[
    ("gnome-terminal", Some("--working-directory=")),
    ("konsole", Some("--workdir ")),
    ("xfce4-terminal", Some("--working-directory=")),
    ("mate-terminal", Some("--working-directory=")),
    ("lxterminal", Some("--working-directory ")),
    ("tilix", Some("-w ")),
    ("terminator", Some("--working-directory ")),
    ("alacritty", Some("--working-directory ")),
    ("kitty", Some("--directory ")),
    ("wezterm", Some("start --cwd ")),
    ("urxvt", Some("-cd ")),
    ("xterm", None),
    ("x-terminal-emulator", Some("--working-directory=")),
];

#[cfg(all(unix, not(target_os = "macos")))]
pub async fn open_in_terminal(dir: &Path) -> Result<()> {
    let path = dir.to_string_lossy().to_string();

    for (program, prefix) in LINUX_TERMINALS {
        if !command_exists(program).await {
            continue;
        }

        let args: Vec<String> = match prefix {
            Some(prefix) => {
                let mut args: Vec<String> = prefix.split_whitespace().map(str::to_string).collect();
                if prefix.ends_with('=') {
                    let last = args.pop().unwrap_or_default();
                    args.push(format!("{last}{path}"));
                } else {
                    args.push(path.clone());
                }
                args
            }
            None => Vec::new(),
        };

        let spawned = Command::new(program)
            .args(&args)
            // Positionné pour tous, pas seulement pour ceux sans option : c'est
            // le filet quand l'option change de nom d'une version à l'autre.
            .current_dir(dir)
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
