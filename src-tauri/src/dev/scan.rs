//! Détection automatique de services dans une arborescence de dépôts.
//!
//! Traduction de `lynk-dev-electron/electron/dev-handlers.ts:706-878`. Les
//! extracteurs de port sont écrits à la main plutôt qu'avec `regex` : ils sont
//! triviaux, et c'est une dépendance de moins à compiler.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tokio::sync::mpsc::UnboundedSender;

use super::types::{ScanProgress, ServiceScanResult, ServiceType};

/// Profondeur maximale explorée sous la racine choisie.
const MAX_DEPTH: usize = 2;
/// Au-delà, on rend ce qu'on a : un scan ne doit jamais bloquer l'écran.
const SCAN_TIMEOUT: Duration = Duration::from_secs(60);
const PROGRESS_THROTTLE: Duration = Duration::from_millis(100);
/// Dossiers qui ne contiennent jamais de service et coûtent cher à parcourir.
const SKIPPED_DIRS: [&str; 5] = ["node_modules", "target", "build", "dist", "__pycache__"];

async fn read_file(path: &Path) -> Option<String> {
    tokio::fs::read_to_string(path).await.ok()
}

async fn exists(path: &Path) -> bool {
    tokio::fs::metadata(path).await.is_ok()
}

fn dir_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

/// `: 8010` ou `= 8010` après un préfixe déjà reconnu.
fn port_after_separator(rest: &str) -> Option<u16> {
    let rest = rest.trim_start();
    let rest = rest.strip_prefix(':').or_else(|| rest.strip_prefix('='))?;
    let digits: String = rest
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// Cherche `server.port: 8010` ou, sous un bloc `server:`, `port: 8010`.
pub fn extract_port_from_yml(content: &str) -> Option<u16> {
    for line in content.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("server.port") {
            if let Some(port) = port_after_separator(rest) {
                return Some(port);
            }
        }
        // Le test du séparateur évite de confondre avec `portfolio:` ou `ports:`.
        if let Some(rest) = trimmed.strip_prefix("port") {
            if let Some(port) = port_after_separator(rest) {
                return Some(port);
            }
        }
    }
    None
}

/// Cherche `server.port=8010` dans un `application.properties`.
pub fn extract_port_from_properties(content: &str) -> Option<u16> {
    for line in content.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("server.port") {
            if let Some(port) = port_after_separator(rest) {
                return Some(port);
            }
        }
    }
    None
}

/// Port d'un service Spring, cherché dans l'ordre de priorité de la version
/// Electron : `application.yml`, puis `application-local.yml`, puis le premier
/// `application-<profil>.yml` venu, puis `application.properties`.
pub async fn detect_spring_port(dir: &Path) -> Option<u16> {
    let resources = dir.join("src").join("main").join("resources");

    for ext in ["yml", "yaml"] {
        if let Some(content) = read_file(&resources.join(format!("application.{ext}"))).await {
            if let Some(port) = extract_port_from_yml(&content) {
                return Some(port);
            }
        }
    }

    for ext in ["yml", "yaml"] {
        if let Some(content) = read_file(&resources.join(format!("application-local.{ext}"))).await
        {
            if let Some(port) = extract_port_from_yml(&content) {
                return Some(port);
            }
        }
    }

    if let Ok(mut entries) = tokio::fs::read_dir(&resources).await {
        let mut candidates: Vec<String> = Vec::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            let profile_yml = name.starts_with("application-")
                && (name.ends_with(".yml") || name.ends_with(".yaml"))
                && !name.starts_with("application-local.");
            if profile_yml {
                candidates.push(name);
            }
        }
        // `read_dir` n'ordonne rien : on trie pour que deux scans successifs
        // rendent le même port.
        candidates.sort();
        if let Some(name) = candidates.first() {
            if let Some(content) = read_file(&resources.join(name)).await {
                if let Some(port) = extract_port_from_yml(&content) {
                    return Some(port);
                }
            }
        }
    }

    if let Some(content) = read_file(&resources.join("application.properties")).await {
        if let Some(port) = extract_port_from_properties(&content) {
            return Some(port);
        }
    }

    None
}

/// Un `pom.xml` qui *mentionne* Spring Boot n'est pas forcément une application :
/// une bibliothèque partagée en dépend tout autant. Seul le **plugin de build**
/// distingue les deux — c'est lui qui produit le jar exécutable.
///
/// Trouvé en recette le 2026-09-05 : `olive_common`, une bibliothèque
/// (`packaging=jar`, aucun `@SpringBootApplication`), était proposée comme
/// service, avec une commande `spring-boot:run` qui n'aurait jamais démarré.
pub fn is_spring_boot_app_maven(pom: &str) -> bool {
    pom.contains("spring-boot-maven-plugin")
}

/// Côté Gradle, c'est la **déclaration du plugin** qui fait foi, pas une
/// dépendance ou un BOM.
pub fn is_spring_boot_app_gradle(build: &str) -> bool {
    build
        .lines()
        .any(|line| line.contains("org.springframework.boot") && line.contains("id"))
}

/// Port lu dans un `Dockerfile` : d'abord un `--port N` de la commande de
/// démarrage (le plus fiable, c'est celui qui sera réellement utilisé), sinon
/// un `EXPOSE N`.
pub fn extract_port_from_dockerfile(content: &str) -> Option<u16> {
    for line in content.lines() {
        let Some(rest) = line.split("--port").nth(1) else {
            continue;
        };
        let digits: String = rest
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(char::is_ascii_digit)
            .collect();
        if let Ok(port) = digits.parse() {
            return Some(port);
        }
    }
    for line in content.lines() {
        let Some(rest) = line.trim_start().strip_prefix("EXPOSE") else {
            continue;
        };
        let digits: String = rest
            .trim_start()
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        if let Ok(port) = digits.parse() {
            return Some(port);
        }
    }
    None
}

/// Point d'entrée ASGI le plus probable, sous la forme `module:app`.
///
/// `None` signifie « ce n'est pas un service web » : proposer `uvicorn
/// main:app` sur un dossier qui n'a pas de `main.py` ne mène nulle part.
async fn detect_asgi_entry(dir: &Path) -> Option<String> {
    for (file, module) in [
        ("app/main.py", "app.main"),
        ("src/main.py", "src.main"),
        ("main.py", "main"),
    ] {
        if exists(&dir.join(file)).await {
            return Some(format!("{module}:app"));
        }
    }
    None
}

/// Reconnaît le type de service d'un dossier, ou `None` s'il n'y en a pas.
pub async fn detect_service(dir: &Path) -> Option<ServiceScanResult> {
    let name = dir_name(dir);
    let is_win = cfg!(windows);

    // Spring Boot (Maven)
    if let Some(pom) = read_file(&dir.join("pom.xml")).await {
        if is_spring_boot_app_maven(&pom) {
            return Some(ServiceScanResult {
                name,
                kind: ServiceType::SpringBootMaven,
                working_dir: dir.to_string_lossy().to_string(),
                suggested_command: if is_win {
                    "mvnw.cmd spring-boot:run".into()
                } else {
                    "./mvnw spring-boot:run".into()
                },
                suggested_build_command: Some(if is_win {
                    "mvnw.cmd clean package -DskipTests".into()
                } else {
                    "./mvnw clean package -DskipTests".into()
                }),
                suggested_port: detect_spring_port(dir).await,
            });
        }
    }

    // Spring Boot (Gradle)
    for build_file in ["build.gradle", "build.gradle.kts"] {
        if let Some(content) = read_file(&dir.join(build_file)).await {
            if is_spring_boot_app_gradle(&content) {
                return Some(ServiceScanResult {
                    name,
                    kind: ServiceType::SpringBootGradle,
                    working_dir: dir.to_string_lossy().to_string(),
                    suggested_command: if is_win {
                        "gradlew.bat bootRun".into()
                    } else {
                        "./gradlew bootRun".into()
                    },
                    suggested_build_command: Some(if is_win {
                        "gradlew.bat clean build -x test".into()
                    } else {
                        "./gradlew clean build -x test".into()
                    }),
                    suggested_port: detect_spring_port(dir).await,
                });
            }
        }
    }

    // Node
    if let Some(raw) = read_file(&dir.join("package.json")).await {
        if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&raw) {
            let scripts = &pkg["scripts"];
            let command = if scripts.get("dev").is_some() {
                Some("npm run dev")
            } else if scripts.get("start").is_some() {
                Some("npm start")
            } else {
                // Pas de script lançable : ce n'est pas un service.
                None
            };
            if let Some(command) = command {
                return Some(ServiceScanResult {
                    name,
                    kind: ServiceType::Node,
                    working_dir: dir.to_string_lossy().to_string(),
                    suggested_command: command.into(),
                    suggested_build_command: scripts
                        .get("build")
                        .map(|_| "npm run build".to_string()),
                    suggested_port: pkg["config"]["port"]
                        .as_u64()
                        .and_then(|p| u16::try_from(p).ok()),
                });
            }
        }
    }

    // Python
    if exists(&dir.join("manage.py")).await {
        return Some(ServiceScanResult {
            name,
            kind: ServiceType::Python,
            working_dir: dir.to_string_lossy().to_string(),
            suggested_command: "python manage.py runserver".into(),
            suggested_build_command: None,
            suggested_port: Some(8000),
        });
    }
    // Un service Python moderne se reconnaît à `pyproject.toml` **ou** au simple
    // `requirements.txt` — trouvé en recette : `olive_ocr_service` n'a que le
    // second, et passait donc totalement inaperçu.
    let has_python_manifest =
        exists(&dir.join("pyproject.toml")).await || exists(&dir.join("requirements.txt")).await;
    if has_python_manifest {
        if let Some(entry) = detect_asgi_entry(dir).await {
            let port = match read_file(&dir.join("Dockerfile")).await {
                Some(dockerfile) => extract_port_from_dockerfile(&dockerfile),
                None => None,
            };
            let command = match port {
                Some(port) => format!("python -m uvicorn {entry} --port {port}"),
                None => format!("python -m uvicorn {entry}"),
            };
            return Some(ServiceScanResult {
                name,
                kind: ServiceType::Python,
                working_dir: dir.to_string_lossy().to_string(),
                suggested_command: command,
                suggested_build_command: None,
                // `uvicorn` écoute sur 8000 quand rien ne le contredit.
                suggested_port: port.or(Some(8000)),
            });
        }
    }

    // Docker Compose
    for compose in ["docker-compose.yml", "docker-compose.yaml", "compose.yml"] {
        if exists(&dir.join(compose)).await {
            return Some(ServiceScanResult {
                name,
                kind: ServiceType::DockerCompose,
                working_dir: dir.to_string_lossy().to_string(),
                suggested_command: format!("docker compose -f {compose} up"),
                suggested_build_command: Some(format!("docker compose -f {compose} build")),
                suggested_port: None,
            });
        }
    }

    None
}

/// Parcourt `root` et rend tous les services reconnus.
///
/// ⚠️ On **ne descend pas** dans un dossier déjà reconnu comme service — sauf
/// pour `docker-compose`, où le compose vit souvent à la racine d'un dossier
/// qui contient par ailleurs les services applicatifs.
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
            let due = last_sent.is_none_or(|t| t.elapsed() >= PROGRESS_THROTTLE);
            if due {
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
            Some(found) => found.kind == ServiceType::DockerCompose,
            None => true,
        };
        if let Some(found) = detected {
            results.push(found);
        }

        if recurse && depth < MAX_DEPTH {
            if let Ok(mut entries) = tokio::fs::read_dir(&dir).await {
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
                children.sort();
                for child in children {
                    queue.push_back((child, depth + 1));
                }
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

    #[test]
    fn yml_reads_a_nested_server_port() {
        let yml = "server:\n  port: 8010\n  servlet:\n    context-path: /\n";
        assert_eq!(extract_port_from_yml(yml), Some(8010));
    }

    #[test]
    fn yml_reads_a_flattened_server_port() {
        assert_eq!(extract_port_from_yml("server.port: 9090\n"), Some(9090));
    }

    /// Le piège : `ports:` d'un compose, ou une clé `portfolio`.
    #[test]
    fn yml_ignores_lookalike_keys() {
        assert_eq!(extract_port_from_yml("portfolio: 42\n"), None);
        assert_eq!(extract_port_from_yml("  ports:\n    - 8010:8010\n"), None);
    }

    #[test]
    fn properties_reads_server_port() {
        assert_eq!(
            extract_port_from_properties("spring.application.name=auth\nserver.port=8010\n"),
            Some(8010)
        );
        assert_eq!(
            extract_port_from_properties("server.port = 8020"),
            Some(8020)
        );
    }

    #[test]
    fn properties_ignores_other_keys() {
        assert_eq!(
            extract_port_from_properties("management.server.port=9000"),
            None
        );
    }

    #[test]
    fn only_the_build_plugin_marks_a_maven_spring_application() {
        // Forme réelle d'`olive_common` : bibliothèque qui dépend de Spring Boot.
        let library = r#"<project>
  <packaging>jar</packaging>
  <dependencies><dependency><groupId>org.springframework.boot</groupId>
  <artifactId>spring-boot-starter</artifactId></dependency></dependencies>
</project>"#;
        assert!(
            !is_spring_boot_app_maven(library),
            "une bibliotheque n'est pas un service"
        );

        let application = r#"<project><build><plugins><plugin>
  <groupId>org.springframework.boot</groupId>
  <artifactId>spring-boot-maven-plugin</artifactId>
</plugin></plugins></build></project>"#;
        assert!(is_spring_boot_app_maven(application));
    }

    #[test]
    fn only_the_plugin_declaration_marks_a_gradle_spring_application() {
        assert!(is_spring_boot_app_gradle(
            "plugins {
  id 'org.springframework.boot' version '3.4.4'
}"
        ));
        assert!(is_spring_boot_app_gradle(
            "plugins {
  id(\"org.springframework.boot\")
}"
        ));
        assert!(
            !is_spring_boot_app_gradle(
                "dependencies {
  implementation 'org.springframework.boot:spring-boot-starter'
}"
            ),
            "une dependance seule ne fait pas une application"
        );
    }

    #[test]
    fn dockerfile_port_prefers_the_start_command_over_expose() {
        let dockerfile = "EXPOSE 9999
CMD [\"uvicorn\", \"app.main:app\", \"--host\", \"0.0.0.0\", \"--port\", \"8120\"]
";
        assert_eq!(extract_port_from_dockerfile(dockerfile), Some(8120));
        assert_eq!(
            extract_port_from_dockerfile(
                "EXPOSE 9999
"
            ),
            Some(9999)
        );
        assert_eq!(
            extract_port_from_dockerfile(
                "--port=8080
"
            ),
            Some(8080)
        );
        assert_eq!(
            extract_port_from_dockerfile(
                "FROM python:3.12-slim
"
            ),
            None
        );
    }

    /// Le service qui manquait à l'appel en recette : un service Python qui n'a
    /// ni `pyproject.toml` ni `manage.py`, seulement un `requirements.txt`.
    #[tokio::test]
    async fn detects_a_python_service_with_only_requirements_txt() {
        let tmp = tempfile::tempdir().expect("tmp");
        let dir = tmp.path().join("olive_ocr_service");
        tokio::fs::create_dir_all(dir.join("app"))
            .await
            .expect("mkdir");
        tokio::fs::write(
            dir.join("requirements.txt"),
            "fastapi
uvicorn
",
        )
        .await
        .expect("req");
        tokio::fs::write(
            dir.join("app").join("main.py"),
            "app = 1
",
        )
        .await
        .expect("main");
        tokio::fs::write(
            dir.join("Dockerfile"),
            "FROM python:3.12-slim
CMD [\"uvicorn\", \"app.main:app\", \"--port\", \"8120\"]
",
        )
        .await
        .expect("dockerfile");

        let found = detect_service(&dir).await.expect("service detecte");
        assert_eq!(found.kind, ServiceType::Python);
        assert_eq!(
            found.suggested_command,
            "python -m uvicorn app.main:app --port 8120"
        );
        assert_eq!(found.suggested_port, Some(8120));
    }

    /// Sans point d'entrée, proposer `uvicorn main:app` ne mènerait nulle part.
    #[tokio::test]
    async fn a_python_package_without_an_entry_point_is_not_a_service() {
        let tmp = tempfile::tempdir().expect("tmp");
        tokio::fs::write(
            tmp.path().join("requirements.txt"),
            "requests
",
        )
        .await
        .expect("req");
        assert!(detect_service(tmp.path()).await.is_none());
    }

    #[tokio::test]
    async fn detects_a_maven_spring_service_with_its_port() {
        let tmp = tempfile::tempdir().expect("tmp");
        let dir = tmp.path().join("olive_auth_service");
        let resources = dir.join("src").join("main").join("resources");
        tokio::fs::create_dir_all(&resources).await.expect("mkdir");
        tokio::fs::write(
            dir.join("pom.xml"),
            "<project>spring-boot-maven-plugin</project>",
        )
        .await
        .expect("pom");
        tokio::fs::write(resources.join("application.yml"), "server:\n  port: 8010\n")
            .await
            .expect("yml");

        let found = detect_service(&dir).await.expect("service detecte");
        assert_eq!(found.kind, ServiceType::SpringBootMaven);
        assert_eq!(found.name, "olive_auth_service");
        assert_eq!(found.suggested_port, Some(8010));
        assert!(found.suggested_build_command.is_some());
    }

    /// Un `pom.xml` sans Spring n'est pas un service lançable.
    #[tokio::test]
    async fn ignores_a_non_spring_pom() {
        let tmp = tempfile::tempdir().expect("tmp");
        tokio::fs::write(tmp.path().join("pom.xml"), "<project>plain maven</project>")
            .await
            .expect("pom");
        assert!(detect_service(tmp.path()).await.is_none());
    }

    /// Un `package.json` sans `dev` ni `start` n'est pas lançable non plus.
    #[tokio::test]
    async fn ignores_a_library_package_json() {
        let tmp = tempfile::tempdir().expect("tmp");
        tokio::fs::write(
            tmp.path().join("package.json"),
            r#"{ "scripts": { "test": "vitest" } }"#,
        )
        .await
        .expect("pkg");
        assert!(detect_service(tmp.path()).await.is_none());
    }

    #[tokio::test]
    async fn prefers_dev_over_start_for_node() {
        let tmp = tempfile::tempdir().expect("tmp");
        tokio::fs::write(
            tmp.path().join("package.json"),
            r#"{ "scripts": { "dev": "vite", "start": "node .", "build": "vite build" } }"#,
        )
        .await
        .expect("pkg");
        let found = detect_service(tmp.path()).await.expect("service");
        assert_eq!(found.kind, ServiceType::Node);
        assert_eq!(found.suggested_command, "npm run dev");
        assert_eq!(
            found.suggested_build_command.as_deref(),
            Some("npm run build")
        );
    }

    #[tokio::test]
    async fn scan_finds_nested_services_and_skips_noise() {
        let tmp = tempfile::tempdir().expect("tmp");
        let back = tmp.path().join("back");
        for name in ["olive_auth_service", "olive_settings_service"] {
            let dir = back.join(name);
            tokio::fs::create_dir_all(&dir).await.expect("mkdir");
            tokio::fs::write(dir.join("pom.xml"), "spring-boot-maven-plugin")
                .await
                .expect("pom");
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
    async fn scan_still_descends_below_a_compose_file() {
        let tmp = tempfile::tempdir().expect("tmp");
        tokio::fs::write(tmp.path().join("docker-compose.yml"), "services: {}")
            .await
            .expect("compose");
        let svc = tmp.path().join("api");
        tokio::fs::create_dir_all(&svc).await.expect("mkdir");
        tokio::fs::write(svc.join("pom.xml"), "spring-boot-maven-plugin")
            .await
            .expect("pom");

        let found = scan_directory(tmp.path(), None).await;
        assert_eq!(found.len(), 2, "compose + service applicatif");
        assert!(found.iter().any(|f| f.kind == ServiceType::DockerCompose));
        assert!(found.iter().any(|f| f.kind == ServiceType::SpringBootMaven));
    }
}
