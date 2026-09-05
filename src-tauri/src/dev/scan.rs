//! Parcours d'une arborescence à la recherche de services.
//!
//! La **reconnaissance** d'un dossier vit dans [`super::detect`] ; ce module ne
//! s'occupe que du parcours : jusqu'où descendre, quoi ignorer, quand s'arrêter.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tokio::sync::mpsc::UnboundedSender;

use super::detect::detect_service;
use super::types::{ScanProgress, ServiceScanResult, ServiceType};

/// Profondeur maximale explorée sous la racine choisie.
const MAX_DEPTH: usize = 2;
/// Au-delà, on rend ce qu'on a : un scan ne doit jamais bloquer l'écran.
const SCAN_TIMEOUT: Duration = Duration::from_secs(60);
const PROGRESS_THROTTLE: Duration = Duration::from_millis(100);
/// Dossiers qui ne contiennent jamais de service et coûtent cher à parcourir.
const SKIPPED_DIRS: [&str; 8] = [
    "node_modules",
    "target",
    "build",
    "dist",
    "__pycache__",
    "vendor",
    "venv",
    ".venv",
];

/// Parcourt `root` et rend tous les services reconnus.
///
/// ⚠️ On **ne descend pas** dans un dossier déjà reconnu comme service — sauf
/// pour `docker-compose`, où le compose vit souvent à la racine d'un dossier qui
/// contient par ailleurs les services applicatifs.
pub async fn scan_directory(
    root: &Path,
    progress: Option<UnboundedSender<ScanProgress>>,
) -> Vec<ServiceScanResult> {
    let deadline = Instant::now() + SCAN_TIMEOUT;
    let mut results: Vec<ServiceScanResult> = Vec::new();
    let mut scanned = 0usize;
    let mut last_sent: Option<Instant> = None;

    let mut queue: VecDeque<(PathBuf, usize)> = VecDeque::new();
    queue.push_back((root.to_path_buf(), 0));

    while let Some((dir, depth)) = queue.pop_front() {
        if Instant::now() > deadline {
            break;
        }
        scanned += 1;

        if let Some(tx) = &progress {
            if last_sent.is_none_or(|sent| sent.elapsed() >= PROGRESS_THROTTLE) {
                last_sent = Some(Instant::now());
                let _ = tx.send(ScanProgress {
                    current: dir.to_string_lossy().to_string(),
                    scanned,
                    found: results.len(),
                });
            }
        }

        let detected = detect_service(&dir).await;
        let recurse = match &detected {
            Some(service) => service.kind == ServiceType::DockerCompose,
            None => true,
        };
        if let Some(service) = detected {
            results.push(service);
        }

        if recurse && depth < MAX_DEPTH {
            let Ok(mut entries) = tokio::fs::read_dir(&dir).await else {
                continue;
            };
            let mut children: Vec<PathBuf> = Vec::new();
            while let Ok(Some(entry)) = entries.next_entry().await {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || SKIPPED_DIRS.contains(&name.as_str()) {
                    continue;
                }
                if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                    children.push(entry.path());
                }
            }
            // Ordre stable d'une exécution à l'autre.
            children.sort();
            for child in children {
                queue.push_back((child, depth + 1));
            }
        }
    }

    if let Some(tx) = &progress {
        let _ = tx.send(ScanProgress {
            current: String::new(),
            scanned,
            found: results.len(),
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn spring_service(dir: &Path) {
        tokio::fs::create_dir_all(dir).await.expect("mkdir");
        tokio::fs::write(dir.join("pom.xml"), "spring-boot-maven-plugin")
            .await
            .expect("pom");
    }

    #[tokio::test]
    async fn finds_nested_services_and_skips_noise() {
        let tmp = tempfile::tempdir().expect("tmp");
        let back = tmp.path().join("back");
        for name in ["olive_auth_service", "olive_settings_service"] {
            spring_service(&back.join(name)).await;
        }
        // Doit être ignoré malgré un package.json lançable.
        let noise = back.join("node_modules").join("some-dep");
        tokio::fs::create_dir_all(&noise).await.expect("mkdir");
        tokio::fs::write(
            noise.join("package.json"),
            r#"{ "scripts": { "start": "x" } }"#,
        )
        .await
        .expect("pkg");

        let found = scan_directory(tmp.path(), None).await;
        let mut names: Vec<&str> = found.iter().map(|f| f.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(names, vec!["olive_auth_service", "olive_settings_service"]);
    }

    /// Un compose à la racine ne doit pas masquer les services en dessous.
    #[tokio::test]
    async fn still_descends_below_a_compose_file() {
        let tmp = tempfile::tempdir().expect("tmp");
        tokio::fs::write(tmp.path().join("docker-compose.yml"), "services: {}")
            .await
            .expect("compose");
        spring_service(&tmp.path().join("api")).await;

        let found = scan_directory(tmp.path(), None).await;
        assert_eq!(found.len(), 2, "compose + service applicatif");
        assert!(found.iter().any(|f| f.kind == ServiceType::DockerCompose));
        assert!(found.iter().any(|f| f.kind == ServiceType::SpringBootMaven));
    }

    /// À l'inverse, on ne descend pas dans un service applicatif : ses
    /// sous-dossiers ne sont pas des services de premier rang.
    #[tokio::test]
    async fn does_not_descend_into_an_application() {
        let tmp = tempfile::tempdir().expect("tmp");
        let app = tmp.path().join("api");
        spring_service(&app).await;
        spring_service(&app.join("module-interne")).await;

        let found = scan_directory(tmp.path(), None).await;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "api");
    }

    #[tokio::test]
    async fn reports_progress_and_a_final_flush() {
        let tmp = tempfile::tempdir().expect("tmp");
        spring_service(&tmp.path().join("api")).await;

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let found = scan_directory(tmp.path(), Some(tx)).await;
        assert_eq!(found.len(), 1);

        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
        let last = events.last().expect("au moins le flush final");
        assert!(
            last.current.is_empty(),
            "le dernier evenement solde le scan"
        );
        assert_eq!(last.found, 1);
    }

    #[tokio::test]
    async fn an_empty_root_yields_nothing() {
        let tmp = tempfile::tempdir().expect("tmp");
        assert!(scan_directory(tmp.path(), None).await.is_empty());
    }
}
