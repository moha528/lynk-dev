//! Reconnaissance d'un service dans un dossier.
//!
//! Point de départ : les six familles de `lynk-dev-electron`. Le catalogue a
//! ensuite été élargi, d'abord parce que la recette du 2026-09-05 a montré
//! qu'un service Python réel passait inaperçu, puis pour couvrir les écosystèmes
//! qu'on croise vraiment aujourd'hui.
//!
//! Deux principes tiennent tout le fichier :
//!
//! 1. **Du plus spécifique au plus générique.** Un projet Next.js est un projet
//!    Node ; le reconnaître comme « Node » serait juste, mais inutile.
//! 2. **Ne rien proposer qui ne démarrerait pas.** Une bibliothèque n'est pas un
//!    service, et un dossier sans point d'entrée non plus. Mieux vaut ne rien
//!    détecter qu'offrir une commande qui échoue.

use std::path::Path;

use super::types::{ServiceScanResult, ServiceType};

// ── Petits utilitaires de lecture ────────────────────────────────────────

pub(super) async fn read_file(path: &Path) -> Option<String> {
    tokio::fs::read_to_string(path).await.ok()
}

pub(super) async fn exists(path: &Path) -> bool {
    tokio::fs::metadata(path).await.is_ok()
}

fn dir_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn found(
    dir: &Path,
    kind: ServiceType,
    command: impl Into<String>,
    build: Option<String>,
    port: Option<u16>,
) -> ServiceScanResult {
    ServiceScanResult {
        name: dir_name(dir),
        kind,
        working_dir: dir.to_string_lossy().to_string(),
        suggested_command: command.into(),
        suggested_build_command: build,
        suggested_port: port,
    }
}

// ── Extraction de ports ──────────────────────────────────────────────────

/// Suite de chiffres qui suit `flag`, en tolérant `=`, guillemets et virgules —
/// ce qui couvre aussi bien `--port 8120` que `"--port","8120"` d'un CMD JSON.
fn digits_after(haystack: &str, flag: &str) -> Option<u16> {
    let rest = haystack.split(flag).nth(1)?;
    let digits: String = rest
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// `--port N`, `--port=N` ou `-p N` dans une ligne de commande.
pub fn extract_port_from_command(command: &str) -> Option<u16> {
    digits_after(command, "--port").or_else(|| digits_after(command, "-p "))
}

/// Port lu dans un `Dockerfile` : d'abord le `--port` de la commande de
/// démarrage (c'est celui qui sera réellement utilisé), sinon un `EXPOSE`.
pub fn extract_port_from_dockerfile(content: &str) -> Option<u16> {
    for line in content.lines() {
        if let Some(port) = digits_after(line, "--port") {
            return Some(port);
        }
    }
    content
        .lines()
        .find_map(|line| line.trim_start().strip_prefix("EXPOSE"))
        .and_then(|rest| {
            let digits: String = rest
                .trim_start()
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            digits.parse().ok()
        })
}

/// `PORT=3000` dans un `.env`.
pub fn extract_port_from_env(content: &str) -> Option<u16> {
    for line in content.lines() {
        let line = line.trim().trim_start_matches("export ");
        let Some(rest) = line.strip_prefix("PORT") else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let digits: String = rest
            .trim()
            .trim_matches(['"', '\''].as_slice())
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        if let Ok(port) = digits.parse() {
            return Some(port);
        }
    }
    None
}

async fn env_port(dir: &Path) -> Option<u16> {
    for name in [".env", ".env.local", ".env.development"] {
        if let Some(content) = read_file(&dir.join(name)).await {
            if let Some(port) = extract_port_from_env(&content) {
                return Some(port);
            }
        }
    }
    None
}

async fn dockerfile_port(dir: &Path) -> Option<u16> {
    extract_port_from_dockerfile(&read_file(&dir.join("Dockerfile")).await?)
}

// ── JVM ──────────────────────────────────────────────────────────────────

/// Cherche `server.port: 8010` ou, sous un bloc `server:`, `port: 8010`.
pub fn extract_port_from_yml(content: &str) -> Option<u16> {
    for line in content.lines() {
        let trimmed = line.trim_start();
        for prefix in ["server.port", "port"] {
            let Some(rest) = trimmed.strip_prefix(prefix) else {
                continue;
            };
            let rest = rest.trim_start();
            // Le test du séparateur évite `portfolio:` ou `ports:`.
            let Some(rest) = rest.strip_prefix(':').or_else(|| rest.strip_prefix('=')) else {
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
    }
    None
}

/// Cherche `server.port=8010` dans un `application.properties`.
pub fn extract_port_from_properties(content: &str) -> Option<u16> {
    content.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix("server.port")?;
        let rest = rest.trim_start().strip_prefix('=')?;
        let digits: String = rest
            .trim_start()
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        digits.parse().ok()
    })
}

/// Un `pom.xml` qui *mentionne* Spring Boot n'est pas forcément une application :
/// une bibliothèque partagée en dépend tout autant. Seul le **plugin de build**
/// distingue les deux — c'est lui qui produit le jar exécutable.
///
/// Trouvé en recette : `olive_common`, une bibliothèque, était proposée comme
/// service avec une commande `spring-boot:run` qui n'aurait jamais démarré.
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

/// Port d'un service Spring, dans l'ordre de priorité : `application.yml`, puis
/// `application-local.yml`, puis le premier profil venu, puis les properties.
pub async fn detect_spring_port(dir: &Path) -> Option<u16> {
    let resources = dir.join("src").join("main").join("resources");

    for name in ["application.yml", "application.yaml"] {
        if let Some(content) = read_file(&resources.join(name)).await {
            if let Some(port) = extract_port_from_yml(&content) {
                return Some(port);
            }
        }
    }
    for name in ["application-local.yml", "application-local.yaml"] {
        if let Some(content) = read_file(&resources.join(name)).await {
            if let Some(port) = extract_port_from_yml(&content) {
                return Some(port);
            }
        }
    }

    if let Ok(mut entries) = tokio::fs::read_dir(&resources).await {
        let mut profiles: Vec<String> = Vec::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("application-")
                && (name.ends_with(".yml") || name.ends_with(".yaml"))
                && !name.starts_with("application-local.")
            {
                profiles.push(name);
            }
        }
        // `read_dir` n'ordonne rien : on trie pour que deux scans successifs
        // proposent le même port.
        profiles.sort();
        if let Some(name) = profiles.first() {
            if let Some(content) = read_file(&resources.join(name)).await {
                if let Some(port) = extract_port_from_yml(&content) {
                    return Some(port);
                }
            }
        }
    }

    read_file(&resources.join("application.properties"))
        .await
        .and_then(|content| extract_port_from_properties(&content))
}

async fn detect_jvm(dir: &Path) -> Option<ServiceScanResult> {
    let windows = cfg!(windows);

    if let Some(pom) = read_file(&dir.join("pom.xml")).await {
        if is_spring_boot_app_maven(&pom) {
            let (run, build) = if windows {
                (
                    "mvnw.cmd spring-boot:run",
                    "mvnw.cmd clean package -DskipTests",
                )
            } else {
                ("./mvnw spring-boot:run", "./mvnw clean package -DskipTests")
            };
            return Some(found(
                dir,
                ServiceType::SpringBootMaven,
                run,
                Some(build.to_string()),
                detect_spring_port(dir).await,
            ));
        }
    }

    for name in ["build.gradle", "build.gradle.kts"] {
        let Some(content) = read_file(&dir.join(name)).await else {
            continue;
        };
        if is_spring_boot_app_gradle(&content) {
            let (run, build) = if windows {
                ("gradlew.bat bootRun", "gradlew.bat clean build -x test")
            } else {
                ("./gradlew bootRun", "./gradlew clean build -x test")
            };
            return Some(found(
                dir,
                ServiceType::SpringBootGradle,
                run,
                Some(build.to_string()),
                detect_spring_port(dir).await,
            ));
        }
    }

    None
}

// ── JavaScript / TypeScript ──────────────────────────────────────────────

/// Gestionnaire de paquets du projet, déduit du fichier de verrou.
///
/// Ça compte : lancer `npm run dev` dans un projet pnpm réinstalle un
/// `node_modules` concurrent et casse les liens existants.
pub async fn package_manager(dir: &Path) -> &'static str {
    if exists(&dir.join("pnpm-lock.yaml")).await {
        "pnpm"
    } else if exists(&dir.join("bun.lockb")).await || exists(&dir.join("bun.lock")).await {
        "bun"
    } else if exists(&dir.join("yarn.lock")).await {
        "yarn"
    } else {
        "npm"
    }
}

/// `npm run dev`, mais `pnpm dev` et `yarn dev` — chaque outil a sa forme
/// idiomatique, et c'est celle que l'utilisateur reconnaîtra.
pub fn run_script(manager: &str, script: &str) -> String {
    match manager {
        "npm" | "bun" => format!("{manager} run {script}"),
        _ => format!("{manager} {script}"),
    }
}

fn has_dependency(pkg: &serde_json::Value, name: &str) -> bool {
    ["dependencies", "devDependencies", "peerDependencies"]
        .iter()
        .any(|section| pkg[*section].get(name).is_some())
}

/// Reconnaît le cadre applicatif et son port d'écoute par défaut.
///
/// L'ordre va du plus spécifique au plus générique : Next, Nuxt et SvelteKit
/// embarquent tous Vite, donc tester Vite en premier les masquerait tous.
pub fn classify_javascript(pkg: &serde_json::Value) -> (ServiceType, Option<u16>) {
    for (dependency, kind, port) in [
        ("next", ServiceType::Next, 3000u16),
        ("nuxt", ServiceType::Nuxt, 3000),
        ("@angular/core", ServiceType::Angular, 4200),
        ("@nestjs/core", ServiceType::Nest, 3000),
        ("@sveltejs/kit", ServiceType::SvelteKit, 5173),
        ("astro", ServiceType::Astro, 4321),
        ("@remix-run/node", ServiceType::Remix, 3000),
        ("@remix-run/react", ServiceType::Remix, 3000),
        ("vite", ServiceType::Vite, 5173),
    ] {
        if has_dependency(pkg, dependency) {
            return (kind, Some(port));
        }
    }
    (ServiceType::Node, None)
}

async fn detect_javascript(dir: &Path) -> Option<ServiceScanResult> {
    let pkg: serde_json::Value =
        serde_json::from_str(&read_file(&dir.join("package.json")).await?).ok()?;

    // Sans script lançable, c'est une bibliothèque : rien à démarrer.
    let script = ["dev", "start", "serve"]
        .into_iter()
        .find(|name| pkg["scripts"].get(name).is_some())?;
    let script_line = pkg["scripts"][script].as_str().unwrap_or_default();

    let manager = package_manager(dir).await;
    let (kind, default_port) = classify_javascript(&pkg);

    // Une application Tauri se lance par son propre orchestrateur : `vite` seul
    // servirait la page web sans jamais ouvrir la fenêtre.
    let command = if exists(&dir.join("src-tauri").join("tauri.conf.json")).await {
        run_script(manager, "tauri dev")
    } else {
        run_script(manager, script)
    };

    let build = pkg["scripts"]
        .get("build")
        .map(|_| run_script(manager, "build"));

    let port = extract_port_from_command(script_line)
        .or(env_port(dir).await)
        .or(default_port);

    Some(found(dir, kind, command, build, port))
}

// ── Python ───────────────────────────────────────────────────────────────

/// Point d'entrée ASGI le plus probable, sous la forme `module:app`.
///
/// `None` signifie « ce n'est pas un service web » : proposer `uvicorn main:app`
/// sur un dossier sans `main.py` ne mène nulle part.
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

async fn detect_python(dir: &Path) -> Option<ServiceScanResult> {
    if exists(&dir.join("manage.py")).await {
        return Some(found(
            dir,
            ServiceType::Django,
            "python manage.py runserver",
            None,
            env_port(dir).await.or(Some(8000)),
        ));
    }

    // `requirements.txt` compte autant que `pyproject.toml` — c'est ce qui
    // manquait pour voir `olive_ocr_service`.
    let manifest = match read_file(&dir.join("pyproject.toml")).await {
        Some(content) => Some(content),
        None => read_file(&dir.join("requirements.txt")).await,
    }?;
    let mentions = |needle: &str| manifest.to_lowercase().contains(needle);

    if mentions("flask") && exists(&dir.join("app.py")).await {
        return Some(found(
            dir,
            ServiceType::Flask,
            "flask run",
            None,
            env_port(dir).await.or(Some(5000)),
        ));
    }

    let entry = detect_asgi_entry(dir).await?;
    let port = dockerfile_port(dir).await.or(env_port(dir).await);
    let command = match port {
        Some(port) => format!("python -m uvicorn {entry} --port {port}"),
        None => format!("python -m uvicorn {entry}"),
    };
    let kind = if mentions("fastapi") {
        ServiceType::Fastapi
    } else {
        ServiceType::Python
    };
    // `uvicorn` écoute sur 8000 quand rien ne le contredit.
    Some(found(dir, kind, command, None, port.or(Some(8000))))
}

// ── Autres écosystèmes ───────────────────────────────────────────────────

async fn detect_go(dir: &Path) -> Option<ServiceScanResult> {
    if !exists(&dir.join("go.mod")).await {
        return None;
    }
    // `./cmd/...` est la disposition idiomatique quand il n'y a pas de `main.go`
    // à la racine.
    let command = if exists(&dir.join("main.go")).await {
        "go run ."
    } else {
        "go run ./cmd/..."
    };
    Some(found(
        dir,
        ServiceType::Go,
        command,
        Some("go build ./...".to_string()),
        env_port(dir).await,
    ))
}

async fn detect_rust(dir: &Path) -> Option<ServiceScanResult> {
    let manifest = read_file(&dir.join("Cargo.toml")).await?;
    // Un manifeste d'espace de travail ou de bibliothèque ne produit aucun
    // binaire : il n'y a rien à lancer.
    let runnable = exists(&dir.join("src").join("main.rs")).await || manifest.contains("[[bin]]");
    if !runnable {
        return None;
    }
    Some(found(
        dir,
        ServiceType::Rust,
        "cargo run",
        Some("cargo build --release".to_string()),
        env_port(dir).await,
    ))
}

async fn detect_dotnet(dir: &Path) -> Option<ServiceScanResult> {
    let mut entries = tokio::fs::read_dir(dir).await.ok()?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        if entry.file_name().to_string_lossy().ends_with(".csproj") {
            return Some(found(
                dir,
                ServiceType::Dotnet,
                "dotnet run",
                Some("dotnet build".to_string()),
                env_port(dir).await,
            ));
        }
    }
    None
}

async fn detect_php(dir: &Path) -> Option<ServiceScanResult> {
    if !exists(&dir.join("artisan")).await {
        return None;
    }
    Some(found(
        dir,
        ServiceType::Laravel,
        "php artisan serve",
        None,
        env_port(dir).await.or(Some(8000)),
    ))
}

async fn detect_ruby(dir: &Path) -> Option<ServiceScanResult> {
    if !exists(&dir.join("bin").join("rails")).await {
        return None;
    }
    Some(found(
        dir,
        ServiceType::Rails,
        "bin/rails server",
        None,
        env_port(dir).await.or(Some(3000)),
    ))
}

async fn detect_compose(dir: &Path) -> Option<ServiceScanResult> {
    for name in ["docker-compose.yml", "docker-compose.yaml", "compose.yml"] {
        if exists(&dir.join(name)).await {
            return Some(found(
                dir,
                ServiceType::DockerCompose,
                format!("docker compose -f {name} up"),
                Some(format!("docker compose -f {name} build")),
                None,
            ));
        }
    }
    None
}

// ── Point d'entrée ───────────────────────────────────────────────────────

/// Reconnaît le service d'un dossier, ou `None` s'il n'y en a pas.
///
/// L'ordre est celui du fichier : JVM, JavaScript, Python, puis les autres
/// écosystèmes, et **`docker-compose` en dernier**. Sans ça, un projet
/// applicatif qui embarque un `docker-compose.yml` pour ses dépendances serait
/// réduit à ses conteneurs.
pub async fn detect_service(dir: &Path) -> Option<ServiceScanResult> {
    if let Some(service) = detect_jvm(dir).await {
        return Some(service);
    }
    if let Some(service) = detect_javascript(dir).await {
        return Some(service);
    }
    if let Some(service) = detect_python(dir).await {
        return Some(service);
    }
    if let Some(service) = detect_go(dir).await {
        return Some(service);
    }
    if let Some(service) = detect_rust(dir).await {
        return Some(service);
    }
    if let Some(service) = detect_dotnet(dir).await {
        return Some(service);
    }
    if let Some(service) = detect_php(dir).await {
        return Some(service);
    }
    if let Some(service) = detect_ruby(dir).await {
        return Some(service);
    }
    detect_compose(dir).await
}
