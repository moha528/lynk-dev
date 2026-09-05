//! Sondes réseau du Dev Manager et mise à mort par port.
//!
//! Traduction de `lynk-dev-electron/electron/dev-handlers.ts:414-500`. Les
//! analyseurs de sortie (`netstat`, `ss`) sont des fonctions **pures**, donc
//! testables sans ouvrir de socket ni tuer quoi que ce soit.

use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};

use crate::process::{run_raw, DEFAULT_TIMEOUT};

fn cwd() -> &'static Path {
    Path::new(".")
}

/// Un port est libre si on peut s'y mettre en écoute sur `127.0.0.1`.
///
/// C'est le test exact de la version Electron (`net.createServer().listen`), et
/// il compte : un simple « est-ce que je peux me connecter ? » répondrait faux
/// pour un port occupé par un process qui n'accepte pas encore.
pub async fn is_port_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).await.is_ok()
}

/// Attend la libération d'un port. Rend `true` s'il est libre avant l'échéance.
pub async fn wait_for_port_free(port: u16, timeout: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + timeout;
    while tokio::time::Instant::now() < deadline {
        if is_port_available(port).await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    is_port_available(port).await
}

/// Le port accepte-t-il une connexion ? Sonde de démarrage d'un service.
pub async fn can_connect(port: u16, timeout: Duration) -> bool {
    matches!(
        tokio::time::timeout(timeout, TcpStream::connect(("127.0.0.1", port))).await,
        Ok(Ok(_))
    )
}

fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

/// Toute réponse 2xx ou 3xx vaut « sain », comme côté Electron
/// (`res.statusCode < 400`). Une erreur réseau vaut « pas sain », jamais une
/// panique : la sonde tourne en boucle sur un service qui démarre.
pub async fn check_health_url(url: &str, timeout: Duration) -> bool {
    match http().get(url).timeout(timeout).send().await {
        Ok(res) => res.status().as_u16() < 400,
        Err(_) => false,
    }
}

/// Extrait les PID en écoute sur `port` d'une sortie `netstat -ano` (Windows).
///
/// Format attendu :
/// `  TCP    0.0.0.0:8010    0.0.0.0:0    LISTENING    1234`
pub fn parse_netstat_pids(output: &str, port: u16) -> Vec<u32> {
    let suffix = format!(":{port}");
    let mut pids = Vec::new();
    for line in output.lines() {
        if !line.contains("LISTENING") {
            continue;
        }
        let cols: Vec<&str> = line.split_whitespace().collect();
        // [proto, local, remote, state, pid]
        if cols.len() < 5 {
            continue;
        }
        if !cols[1].ends_with(&suffix) {
            continue;
        }
        if let Ok(pid) = cols[cols.len() - 1].parse::<u32>() {
            // Le PID 0 est le process « System Idle » : le tuer n'a aucun sens.
            if pid != 0 && !pids.contains(&pid) {
                pids.push(pid);
            }
        }
    }
    pids
}

/// Extrait les PID d'une sortie `ss -tlnpH` (Linux), qui les expose en
/// `users:(("java",pid=1234,fd=42))`.
pub fn parse_ss_pids(output: &str) -> Vec<u32> {
    let mut pids = Vec::new();
    for chunk in output.split("pid=").skip(1) {
        let digits: String = chunk.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(pid) = digits.parse::<u32>() {
            if !pids.contains(&pid) {
                pids.push(pid);
            }
        }
    }
    pids
}

/// Extrait les PID d'une sortie `lsof -ti:PORT` (un PID par ligne).
pub fn parse_lsof_pids(output: &str) -> Vec<u32> {
    output
        .lines()
        .filter_map(|l| l.trim().parse::<u32>().ok())
        .collect()
}

/// Tue ce qui écoute sur `port`, quel que soit le propriétaire.
///
/// Dernier recours quand un service « externe » (ou un orphelin d'une session
/// précédente) squatte le port qu'on veut réserver.
pub async fn kill_by_port(port: u16) -> bool {
    #[cfg(windows)]
    {
        let Ok(out) = run_raw(cwd(), "netstat", &["-ano"], DEFAULT_TIMEOUT).await else {
            return false;
        };
        let pids = parse_netstat_pids(&out.stdout, port);
        if pids.is_empty() {
            return false;
        }
        let mut killed = false;
        for pid in pids {
            let pid = pid.to_string();
            // `/T` tue l'arbre : sans lui, `cmd.exe` meurt et `java` survit.
            if run_raw(
                cwd(),
                "taskkill",
                &["/pid", &pid, "/T", "/F"],
                DEFAULT_TIMEOUT,
            )
            .await
            .is_ok()
            {
                killed = true;
            }
        }
        killed
    }

    #[cfg(unix)]
    {
        // L'ordre compte : les images minimales (Alpine, conteneurs sans
        // outils) n'ont pas `lsof`, et `ss` n'existe pas sur macOS.
        let mut pids: Vec<u32> = Vec::new();

        if let Ok(out) = run_raw(cwd(), "lsof", &[&format!("-ti:{port}")], DEFAULT_TIMEOUT).await {
            pids = parse_lsof_pids(&out.stdout);
        }
        if pids.is_empty() {
            if let Ok(out) = run_raw(
                cwd(),
                "ss",
                &["-tlnpH", &format!("sport = :{port}")],
                DEFAULT_TIMEOUT,
            )
            .await
            {
                pids = parse_ss_pids(&out.stdout);
            }
        }

        if !pids.is_empty() {
            let args: Vec<String> = std::iter::once("-9".to_string())
                .chain(pids.iter().map(|p| p.to_string()))
                .collect();
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            return run_raw(cwd(), "kill", &args, DEFAULT_TIMEOUT)
                .await
                .map(|o| o.ok())
                .unwrap_or(false);
        }

        // Ultime recours : `fuser` tue sans jamais exposer les PID.
        run_raw(
            cwd(),
            "fuser",
            &["-k", "-n", "tcp", &port.to_string()],
            DEFAULT_TIMEOUT,
        )
        .await
        .map(|o| o.ok())
        .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn netstat_keeps_only_the_matching_listening_port() {
        let out = "\
  Proto  Adresse locale         Adresse distante       Etat            PID
  TCP    0.0.0.0:8010           0.0.0.0:0              LISTENING       1234
  TCP    127.0.0.1:8010         0.0.0.0:0              LISTENING       1234
  TCP    0.0.0.0:8020           0.0.0.0:0              LISTENING       5678
  TCP    127.0.0.1:8010         127.0.0.1:51000        ESTABLISHED     9999
";
        assert_eq!(parse_netstat_pids(out, 8010), vec![1234]);
        assert_eq!(parse_netstat_pids(out, 8020), vec![5678]);
        assert!(parse_netstat_pids(out, 9090).is_empty());
    }

    /// `:8010` ne doit pas matcher `:18010`.
    #[test]
    fn netstat_does_not_match_a_port_suffix() {
        let out = "  TCP    0.0.0.0:18010    0.0.0.0:0    LISTENING    42\n";
        assert!(parse_netstat_pids(out, 8010).is_empty());
        assert_eq!(parse_netstat_pids(out, 18010), vec![42]);
    }

    /// Le PID 0 (System Idle) ne doit jamais remonter.
    #[test]
    fn netstat_skips_pid_zero() {
        let out = "  TCP    0.0.0.0:8010    0.0.0.0:0    LISTENING    0\n";
        assert!(parse_netstat_pids(out, 8010).is_empty());
    }

    #[test]
    fn ss_extracts_and_dedupes_pids() {
        let out = r#"LISTEN 0 4096 *:8010 *:* users:(("java",pid=1234,fd=42),("java",pid=1234,fd=43))
LISTEN 0 4096 *:8020 *:* users:(("node",pid=5678,fd=20))"#;
        assert_eq!(parse_ss_pids(out), vec![1234, 5678]);
    }

    #[test]
    fn lsof_reads_one_pid_per_line() {
        assert_eq!(parse_lsof_pids("1234\n5678\n"), vec![1234, 5678]);
        assert!(parse_lsof_pids("").is_empty());
    }

    /// Un port qu'on vient d'ouvrir n'est pas disponible ; une fois relâché, il
    /// l'est de nouveau.
    #[tokio::test]
    async fn port_availability_follows_a_real_listener() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind");
        let port = listener.local_addr().expect("addr").port();
        assert!(!is_port_available(port).await, "port pris = indisponible");
        drop(listener);
        assert!(
            wait_for_port_free(port, Duration::from_secs(2)).await,
            "port relache = disponible"
        );
    }

    #[tokio::test]
    async fn health_check_is_false_on_an_unreachable_url() {
        // Port fermé : la sonde doit rendre `false`, pas paniquer.
        assert!(!check_health_url("http://127.0.0.1:1/health", Duration::from_millis(500)).await);
    }
}
