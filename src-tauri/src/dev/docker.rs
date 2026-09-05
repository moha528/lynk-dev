//! Sondes `docker compose`.
//!
//! Traduction de `lynk-dev-electron/electron/dev-handlers.ts:626-700` et du
//! handler `dev:docker:health` (`:1331-1376`).
//!
//! ⚠️ Le repli en **texte brut** n'est pas décoratif : `--format json` n'existe
//! pas sur les vieux Docker, et sans lui un compose parfaitement démarré serait
//! rendu « down ».

use std::path::Path;
use std::time::Duration;

use crate::process::run_raw;

use super::types::{DockerContainer, DockerHealth, DockerHealthReport};

const DOCKER_TIMEOUT: Duration = Duration::from_secs(10);

/// Analyse la sortie de `docker compose ps --format json`.
///
/// Deux formats coexistent selon la version de Docker : **un objet JSON par
/// ligne** (historique) ou **un tableau JSON** (récent). Les deux sont acceptés.
pub fn parse_compose_ps_json(stdout: &str) -> Vec<DockerContainer> {
    let field = |value: &serde_json::Value, keys: &[&str]| -> String {
        for key in keys {
            if let Some(found) = value.get(*key).and_then(|v| v.as_str()) {
                if !found.is_empty() {
                    return found.to_string();
                }
            }
        }
        String::new()
    };
    let to_container = |value: &serde_json::Value| DockerContainer {
        name: field(value, &["Name", "Service"]),
        state: field(value, &["State"]).to_lowercase(),
        health: field(value, &["Health"]).to_lowercase(),
    };

    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if let Ok(serde_json::Value::Array(items)) = serde_json::from_str::<serde_json::Value>(trimmed)
    {
        return items.iter().map(to_container).collect();
    }

    trimmed
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line.trim()).ok())
        .map(|v| to_container(&v))
        .collect()
}

/// Repli texte : `docker compose ps` sans format machine. On cherche `running`
/// ou `Up` dans les colonnes d'état.
pub fn text_output_has_running(stdout: &str) -> bool {
    stdout.lines().any(|line| {
        let lower = line.to_lowercase();
        lower.split_whitespace().any(|w| w == "running")
            || line.split_whitespace().any(|w| w == "Up")
    })
}

/// Au moins un conteneur du compose tourne-t-il ?
pub async fn compose_running(working_dir: &Path, compose_file: &str) -> bool {
    if let Ok(out) = run_raw(
        working_dir,
        "docker",
        &["compose", "-f", compose_file, "ps", "--format", "json"],
        DOCKER_TIMEOUT,
    )
    .await
    {
        if out.ok() {
            let containers = parse_compose_ps_json(&out.stdout);
            if !containers.is_empty() {
                return containers.iter().any(|c| c.state == "running");
            }
        }
    }

    match run_raw(
        working_dir,
        "docker",
        &["compose", "-f", compose_file, "ps"],
        DOCKER_TIMEOUT,
    )
    .await
    {
        Ok(out) if out.ok() => text_output_has_running(&out.stdout),
        _ => false,
    }
}

/// État détaillé d'un compose, pour l'affichage.
pub async fn compose_health(working_dir: &Path, compose_file: &str) -> DockerHealthReport {
    let Ok(out) = run_raw(
        working_dir,
        "docker",
        &["compose", "-f", compose_file, "ps", "--format", "json"],
        DOCKER_TIMEOUT,
    )
    .await
    else {
        return DockerHealthReport {
            status: DockerHealth::Down,
            services: Vec::new(),
        };
    };

    let services = parse_compose_ps_json(&out.stdout);
    if services.is_empty() {
        return DockerHealthReport {
            status: DockerHealth::Down,
            services,
        };
    }

    let all = services.iter().all(|c| c.state == "running");
    let some = services.iter().any(|c| c.state == "running");
    DockerHealthReport {
        status: if all {
            DockerHealth::Up
        } else if some {
            DockerHealth::Partial
        } else {
            DockerHealth::Down
        },
        services,
    }
}

/// Arrête un compose (service « externe » que nous ne supervisons pas).
pub async fn compose_stop(working_dir: &Path, compose_file: &str) -> bool {
    run_raw(
        working_dir,
        "docker",
        &["compose", "-f", compose_file, "stop"],
        Duration::from_secs(30),
    )
    .await
    .map(|o| o.ok())
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_one_json_object_per_line() {
        let out = r#"{"Name":"pg","State":"running","Health":"healthy"}
{"Name":"consul","State":"exited","Health":""}"#;
        let containers = parse_compose_ps_json(out);
        assert_eq!(containers.len(), 2);
        assert_eq!(containers[0].name, "pg");
        assert_eq!(containers[0].state, "running");
        assert_eq!(containers[1].state, "exited");
    }

    /// Docker récent rend un tableau : ne pas le rendre « down » pour autant.
    #[test]
    fn parses_a_json_array() {
        let out = r#"[{"Name":"pg","State":"running"},{"Name":"consul","State":"running"}]"#;
        let containers = parse_compose_ps_json(out);
        assert_eq!(containers.len(), 2);
        assert!(containers.iter().all(|c| c.state == "running"));
    }

    #[test]
    fn falls_back_to_the_service_key_when_name_is_absent() {
        let containers = parse_compose_ps_json(r#"{"Service":"adminer","State":"Running"}"#);
        assert_eq!(containers[0].name, "adminer");
        assert_eq!(containers[0].state, "running", "l'etat est normalise");
    }

    #[test]
    fn empty_output_yields_no_container() {
        assert!(parse_compose_ps_json("").is_empty());
        assert!(parse_compose_ps_json("   \n").is_empty());
    }

    #[test]
    fn text_fallback_detects_both_wordings() {
        assert!(text_output_has_running(
            "NAME   IMAGE   STATUS\npg   postgres   running   5600/tcp"
        ));
        assert!(text_output_has_running("pg   postgres   Up 3 minutes"));
        assert!(!text_output_has_running(
            "pg   postgres   Exited (0) 2 min ago"
        ));
    }

    /// « Up » ne doit pas matcher au milieu d'un mot.
    #[test]
    fn text_fallback_does_not_match_a_substring() {
        assert!(!text_output_has_running("NAME   IMAGE   Updating"));
    }
}
