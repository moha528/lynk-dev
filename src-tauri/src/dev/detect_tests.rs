//! Tests du catalogue de reconnaissance.
//!
//! Séparés de [`super::detect`] parce qu'ils sont volumineux et que la lecture
//! du catalogue lui-même doit rester d'un seul tenant. Tout ce qu'ils
//! sollicitent est public : aucun accès privilégié n'est nécessaire.

use std::path::Path;

use super::detect::*;
use super::types::ServiceType;

async fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.expect("mkdir");
    }
    tokio::fs::write(path, content).await.expect("write");
}

// ── Extraction de ports ──────────────────────────────────────────────────

#[test]
fn port_from_command_reads_both_flags() {
    assert_eq!(
        extract_port_from_command("next dev --port 3001"),
        Some(3001)
    );
    assert_eq!(extract_port_from_command("vite --port=5174"), Some(5174));
    assert_eq!(
        extract_port_from_command("node server.js -p 4000"),
        Some(4000)
    );
    assert_eq!(extract_port_from_command("next dev"), None);
}

#[test]
fn dockerfile_port_prefers_the_start_command_over_expose() {
    let dockerfile = "EXPOSE 9999\nCMD [\"uvicorn\", \"app.main:app\", \"--port\", \"8120\"]\n";
    assert_eq!(extract_port_from_dockerfile(dockerfile), Some(8120));
    assert_eq!(extract_port_from_dockerfile("EXPOSE 9999\n"), Some(9999));
    assert_eq!(
        extract_port_from_dockerfile("FROM python:3.12-slim\n"),
        None
    );
}

#[test]
fn env_port_tolerates_quotes_and_export() {
    assert_eq!(extract_port_from_env("PORT=3000"), Some(3000));
    assert_eq!(extract_port_from_env("export PORT=8080\n"), Some(8080));
    assert_eq!(extract_port_from_env("PORT = \"4321\"\n"), Some(4321));
    // `PORTAIL` et `DB_PORT` ne doivent pas passer pour `PORT`.
    assert_eq!(extract_port_from_env("PORTAIL=1\nDB_PORT=5432\n"), None);
}

#[test]
fn yml_reads_nested_and_flattened_server_port() {
    assert_eq!(extract_port_from_yml("server:\n  port: 8010\n"), Some(8010));
    assert_eq!(extract_port_from_yml("server.port: 9090\n"), Some(9090));
    assert_eq!(extract_port_from_yml("portfolio: 42\n"), None);
    assert_eq!(extract_port_from_yml("  ports:\n    - 8010:8010\n"), None);
}

#[test]
fn properties_reads_only_the_server_port_key() {
    assert_eq!(extract_port_from_properties("server.port=8010"), Some(8010));
    assert_eq!(
        extract_port_from_properties("management.server.port=9000"),
        None
    );
}

// ── JVM ──────────────────────────────────────────────────────────────────

#[test]
fn only_the_build_plugin_marks_a_maven_spring_application() {
    let library = "<project><packaging>jar</packaging>spring-boot-starter</project>";
    assert!(
        !is_spring_boot_app_maven(library),
        "une bibliotheque n'est pas un service"
    );
    assert!(is_spring_boot_app_maven(
        "<plugin>spring-boot-maven-plugin</plugin>"
    ));
}

#[test]
fn only_the_plugin_declaration_marks_a_gradle_spring_application() {
    assert!(is_spring_boot_app_gradle(
        "plugins { id 'org.springframework.boot' version '3.4.4' }"
    ));
    assert!(!is_spring_boot_app_gradle(
        "dependencies { implementation 'org.springframework.boot:spring-boot-starter' }"
    ));
}

#[tokio::test]
async fn detects_a_maven_spring_service_with_its_port() {
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = tmp.path().join("olive_auth_service");
    write(&dir.join("pom.xml"), "spring-boot-maven-plugin").await;
    write(
        &dir.join("src")
            .join("main")
            .join("resources")
            .join("application.yml"),
        "server:\n  port: 8010\n",
    )
    .await;

    let service = detect_service(&dir).await.expect("service");
    assert_eq!(service.kind, ServiceType::SpringBootMaven);
    assert_eq!(service.suggested_port, Some(8010));
    assert!(service.suggested_build_command.is_some());
}

// ── JavaScript ───────────────────────────────────────────────────────────

#[tokio::test]
async fn package_manager_follows_the_lockfile() {
    let tmp = tempfile::tempdir().expect("tmp");
    assert_eq!(
        package_manager(tmp.path()).await,
        "npm",
        "aucun verrou = npm"
    );

    write(&tmp.path().join("yarn.lock"), "").await;
    assert_eq!(package_manager(tmp.path()).await, "yarn");

    write(&tmp.path().join("pnpm-lock.yaml"), "").await;
    assert_eq!(package_manager(tmp.path()).await, "pnpm", "pnpm l'emporte");
}

#[test]
fn run_script_uses_each_tool_idiom() {
    assert_eq!(run_script("npm", "dev"), "npm run dev");
    assert_eq!(run_script("pnpm", "dev"), "pnpm dev");
    assert_eq!(run_script("yarn", "dev"), "yarn dev");
    assert_eq!(run_script("bun", "dev"), "bun run dev");
}

/// Next, Nuxt et SvelteKit embarquent tous Vite : tester Vite en premier les
/// masquerait tous.
#[test]
fn framework_classification_goes_from_specific_to_generic() {
    let with = |deps: serde_json::Value| serde_json::json!({ "dependencies": deps });

    assert_eq!(
        classify_javascript(&with(serde_json::json!({ "next": "15", "vite": "6" }))).0,
        ServiceType::Next
    );
    assert_eq!(
        classify_javascript(&with(
            serde_json::json!({ "@sveltejs/kit": "2", "vite": "6" })
        ))
        .0,
        ServiceType::SvelteKit
    );
    assert_eq!(
        classify_javascript(&with(serde_json::json!({ "@angular/core": "21" }))),
        (ServiceType::Angular, Some(4200))
    );
    assert_eq!(
        classify_javascript(&with(serde_json::json!({ "@nestjs/core": "10" }))).0,
        ServiceType::Nest
    );
    assert_eq!(
        classify_javascript(&with(serde_json::json!({ "astro": "5" }))),
        (ServiceType::Astro, Some(4321))
    );
    assert_eq!(
        classify_javascript(&with(serde_json::json!({ "vite": "6" }))),
        (ServiceType::Vite, Some(5173))
    );
    assert_eq!(
        classify_javascript(&with(serde_json::json!({ "express": "4" }))),
        (ServiceType::Node, None)
    );
}

#[tokio::test]
async fn detects_a_next_project_with_its_package_manager() {
    let tmp = tempfile::tempdir().expect("tmp");
    write(&tmp.path().join("pnpm-lock.yaml"), "").await;
    write(
        &tmp.path().join("package.json"),
        r#"{ "scripts": { "dev": "next dev", "build": "next build" },
             "dependencies": { "next": "15.0.0" } }"#,
    )
    .await;

    let service = detect_service(tmp.path()).await.expect("service");
    assert_eq!(service.kind, ServiceType::Next);
    assert_eq!(service.suggested_command, "pnpm dev");
    assert_eq!(
        service.suggested_build_command.as_deref(),
        Some("pnpm build")
    );
    assert_eq!(service.suggested_port, Some(3000));
}

/// Un port explicite dans le script prime sur le defaut du cadre.
#[tokio::test]
async fn a_port_in_the_script_wins_over_the_framework_default() {
    let tmp = tempfile::tempdir().expect("tmp");
    write(
        &tmp.path().join("package.json"),
        r#"{ "scripts": { "dev": "vite --port 5180" }, "devDependencies": { "vite": "6" } }"#,
    )
    .await;

    let service = detect_service(tmp.path()).await.expect("service");
    assert_eq!(service.suggested_port, Some(5180));
}

#[tokio::test]
async fn env_port_wins_over_the_framework_default() {
    let tmp = tempfile::tempdir().expect("tmp");
    write(&tmp.path().join(".env"), "PORT=4500\n").await;
    write(
        &tmp.path().join("package.json"),
        r#"{ "scripts": { "dev": "next dev" }, "dependencies": { "next": "15" } }"#,
    )
    .await;

    let service = detect_service(tmp.path()).await.expect("service");
    assert_eq!(service.suggested_port, Some(4500));
}

#[tokio::test]
async fn a_library_package_json_is_not_a_service() {
    let tmp = tempfile::tempdir().expect("tmp");
    write(
        &tmp.path().join("package.json"),
        r#"{ "scripts": { "test": "vitest", "build": "tsup" } }"#,
    )
    .await;
    assert!(detect_service(tmp.path()).await.is_none());
}

/// Une application Tauri se lance par son orchestrateur : `vite` seul servirait
/// la page web sans jamais ouvrir la fenetre.
#[tokio::test]
async fn a_tauri_project_is_launched_through_tauri() {
    let tmp = tempfile::tempdir().expect("tmp");
    write(&tmp.path().join("pnpm-lock.yaml"), "").await;
    write(
        &tmp.path().join("package.json"),
        r#"{ "scripts": { "dev": "vite" }, "devDependencies": { "vite": "6" } }"#,
    )
    .await;
    write(&tmp.path().join("src-tauri").join("tauri.conf.json"), "{}").await;

    let service = detect_service(tmp.path()).await.expect("service");
    assert_eq!(service.suggested_command, "pnpm tauri dev");
}

// ── Python ───────────────────────────────────────────────────────────────

/// Le service qui manquait a l'appel en recette : ni `pyproject.toml`, ni
/// `manage.py`, seulement un `requirements.txt`.
#[tokio::test]
async fn detects_a_fastapi_service_with_only_requirements_txt() {
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = tmp.path().join("olive_ocr_service");
    write(&dir.join("requirements.txt"), "fastapi\nuvicorn\n").await;
    write(&dir.join("app").join("main.py"), "app = 1\n").await;
    write(
        &dir.join("Dockerfile"),
        "FROM python:3.12-slim\nCMD [\"uvicorn\", \"app.main:app\", \"--port\", \"8120\"]\n",
    )
    .await;

    let service = detect_service(&dir).await.expect("service");
    assert_eq!(service.kind, ServiceType::Fastapi);
    assert_eq!(
        service.suggested_command,
        "python -m uvicorn app.main:app --port 8120"
    );
    assert_eq!(service.suggested_port, Some(8120));
}

#[tokio::test]
async fn a_python_package_without_an_entry_point_is_not_a_service() {
    let tmp = tempfile::tempdir().expect("tmp");
    write(&tmp.path().join("requirements.txt"), "requests\n").await;
    assert!(detect_service(tmp.path()).await.is_none());
}

#[tokio::test]
async fn detects_django_and_flask() {
    let django = tempfile::tempdir().expect("tmp");
    write(&django.path().join("manage.py"), "").await;
    let service = detect_service(django.path()).await.expect("django");
    assert_eq!(service.kind, ServiceType::Django);
    assert_eq!(service.suggested_port, Some(8000));

    let flask = tempfile::tempdir().expect("tmp");
    write(&flask.path().join("requirements.txt"), "Flask==3.0\n").await;
    write(&flask.path().join("app.py"), "app = 1\n").await;
    let service = detect_service(flask.path()).await.expect("flask");
    assert_eq!(service.kind, ServiceType::Flask);
    assert_eq!(service.suggested_port, Some(5000));
}

// ── Autres ecosystemes ───────────────────────────────────────────────────

#[tokio::test]
async fn detects_go_rust_dotnet_laravel_and_rails() {
    let go = tempfile::tempdir().expect("tmp");
    write(&go.path().join("go.mod"), "module x\n").await;
    write(&go.path().join("main.go"), "package main\n").await;
    let service = detect_service(go.path()).await.expect("go");
    assert_eq!(service.kind, ServiceType::Go);
    assert_eq!(service.suggested_command, "go run .");

    let rust = tempfile::tempdir().expect("tmp");
    write(&rust.path().join("Cargo.toml"), "[package]\n").await;
    write(&rust.path().join("src").join("main.rs"), "fn main() {}\n").await;
    assert_eq!(
        detect_service(rust.path()).await.expect("rust").kind,
        ServiceType::Rust
    );

    let dotnet = tempfile::tempdir().expect("tmp");
    write(&dotnet.path().join("Api.csproj"), "<Project/>").await;
    assert_eq!(
        detect_service(dotnet.path()).await.expect("dotnet").kind,
        ServiceType::Dotnet
    );

    let laravel = tempfile::tempdir().expect("tmp");
    write(&laravel.path().join("artisan"), "").await;
    assert_eq!(
        detect_service(laravel.path()).await.expect("laravel").kind,
        ServiceType::Laravel
    );

    let rails = tempfile::tempdir().expect("tmp");
    write(&rails.path().join("bin").join("rails"), "").await;
    assert_eq!(
        detect_service(rails.path()).await.expect("rails").kind,
        ServiceType::Rails
    );
}

/// Un espace de travail Cargo ne produit aucun binaire : rien a lancer.
#[tokio::test]
async fn a_cargo_workspace_without_a_binary_is_not_a_service() {
    let tmp = tempfile::tempdir().expect("tmp");
    write(
        &tmp.path().join("Cargo.toml"),
        "[workspace]\nmembers = []\n",
    )
    .await;
    assert!(detect_service(tmp.path()).await.is_none());
}

// ── Ordre du catalogue ───────────────────────────────────────────────────

/// Un projet applicatif qui embarque un compose pour ses dependances reste un
/// projet applicatif.
#[tokio::test]
async fn an_application_wins_over_its_own_compose_file() {
    let tmp = tempfile::tempdir().expect("tmp");
    write(&tmp.path().join("docker-compose.yml"), "services: {}").await;
    write(
        &tmp.path().join("package.json"),
        r#"{ "scripts": { "dev": "next dev" }, "dependencies": { "next": "15" } }"#,
    )
    .await;

    assert_eq!(
        detect_service(tmp.path()).await.expect("service").kind,
        ServiceType::Next
    );
}

#[tokio::test]
async fn a_folder_with_only_a_compose_file_is_a_compose_service() {
    let tmp = tempfile::tempdir().expect("tmp");
    write(&tmp.path().join("docker-compose.yml"), "services: {}").await;

    let service = detect_service(tmp.path()).await.expect("service");
    assert_eq!(service.kind, ServiceType::DockerCompose);
    assert_eq!(
        service.suggested_command,
        "docker compose -f docker-compose.yml up"
    );
}
