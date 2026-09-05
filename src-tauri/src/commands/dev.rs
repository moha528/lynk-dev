//! Pont IPC du Dev Manager.
//!
//! ⚠️ **Aucune logique métier ici.** Ces commandes lisent le profil, appellent
//! le superviseur, rendent le résultat. Toute règle glissée à ce niveau serait
//! invisible du futur serveur MCP, qui s'adressera directement au superviseur
//! — c'est exactement ce que le principe directeur du chantier interdit.

use std::path::Path;
use std::sync::Arc;

use tauri::{Emitter, State};

use crate::dev::logs::LogStore;
use crate::dev::types::{
    DevProfile, DockerHealthReport, ManagedProcessInfo, PortCheckResult, PortRequest, ProbeResult,
    ScanProgress, ServiceScanResult,
};
use crate::dev::{batch, detect, docker, net, scan, StartOptions, Supervisor};
use crate::store::{dev_profiles as dao, DbPool};
use crate::AppError;

async fn load_profile(pool: &DbPool, profile_id: &str) -> Result<DevProfile, AppError> {
    dao::get(pool, profile_id)
        .await?
        .ok_or_else(|| AppError(anyhow::anyhow!("profil {profile_id} introuvable")))
}

// ── Profils ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn dev_profile_list(pool: State<'_, DbPool>) -> Result<Vec<DevProfile>, AppError> {
    Ok(dao::all(pool.inner()).await?)
}

#[tauri::command]
pub async fn dev_profile_save(
    pool: State<'_, DbPool>,
    profile: DevProfile,
) -> Result<(), AppError> {
    dao::save(pool.inner(), &profile).await?;
    Ok(())
}

#[tauri::command]
pub async fn dev_profile_delete(
    pool: State<'_, DbPool>,
    profile_id: String,
) -> Result<(), AppError> {
    dao::delete(pool.inner(), &profile_id).await?;
    Ok(())
}

// ── Détection ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn dev_scan(
    app: tauri::AppHandle,
    root_path: String,
) -> Result<Vec<ServiceScanResult>, AppError> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<ScanProgress>();
    let forwarder = tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            let _ = app.emit("dev:scan:progress", progress);
        }
    });

    let results = scan::scan_directory(Path::new(&root_path), Some(tx)).await;
    // `tx` est consommé par le scan : sa fin ferme le canal, donc la tâche.
    let _ = forwarder.await;
    Ok(results)
}

#[tauri::command]
pub async fn dev_detect(dir_path: String) -> Result<Option<ServiceScanResult>, AppError> {
    Ok(detect::detect_service(Path::new(&dir_path)).await)
}

// ── Cycle de vie ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn dev_service_start(
    pool: State<'_, DbPool>,
    supervisor: State<'_, Arc<Supervisor>>,
    profile_id: String,
    service_id: String,
) -> Result<bool, AppError> {
    let profile = load_profile(pool.inner(), &profile_id).await?;
    let Some(config) = profile.service(&service_id).cloned() else {
        return Ok(false);
    };
    supervisor
        .inner()
        .start(profile_id, config, StartOptions::default())
        .await;
    Ok(true)
}

#[tauri::command]
pub async fn dev_service_stop(
    pool: State<'_, DbPool>,
    supervisor: State<'_, Arc<Supervisor>>,
    profile_id: String,
    service_id: String,
) -> Result<bool, AppError> {
    let profile = load_profile(pool.inner(), &profile_id).await?;
    let Some(config) = profile.service(&service_id).cloned() else {
        return Ok(false);
    };
    supervisor.stop(&profile_id, &config).await;
    Ok(true)
}

#[tauri::command]
pub async fn dev_service_restart(
    pool: State<'_, DbPool>,
    supervisor: State<'_, Arc<Supervisor>>,
    profile_id: String,
    service_id: String,
) -> Result<bool, AppError> {
    let profile = load_profile(pool.inner(), &profile_id).await?;
    let Some(config) = profile.service(&service_id).cloned() else {
        return Ok(false);
    };
    supervisor.inner().restart(&profile_id, &config).await;
    Ok(true)
}

#[tauri::command]
pub async fn dev_service_build(
    pool: State<'_, DbPool>,
    supervisor: State<'_, Arc<Supervisor>>,
    profile_id: String,
    service_id: String,
) -> Result<bool, AppError> {
    let profile = load_profile(pool.inner(), &profile_id).await?;
    let Some(config) = profile.service(&service_id).cloned() else {
        return Ok(false);
    };
    Ok(supervisor.build(&config).await)
}

// ── Opérations groupées ──────────────────────────────────────────────────

#[tauri::command]
pub async fn dev_service_start_batch(
    pool: State<'_, DbPool>,
    supervisor: State<'_, Arc<Supervisor>>,
    profile_id: String,
    service_ids: Vec<String>,
) -> Result<bool, AppError> {
    let profile = load_profile(pool.inner(), &profile_id).await?;
    batch::start(supervisor.inner(), &profile, &service_ids).await;
    Ok(true)
}

#[tauri::command]
pub async fn dev_service_stop_batch(
    pool: State<'_, DbPool>,
    supervisor: State<'_, Arc<Supervisor>>,
    profile_id: String,
    service_ids: Vec<String>,
) -> Result<bool, AppError> {
    let profile = load_profile(pool.inner(), &profile_id).await?;
    batch::stop(supervisor.inner(), &profile, &service_ids).await;
    Ok(true)
}

#[tauri::command]
pub async fn dev_service_restart_batch(
    pool: State<'_, DbPool>,
    supervisor: State<'_, Arc<Supervisor>>,
    profile_id: String,
    service_ids: Vec<String>,
) -> Result<bool, AppError> {
    let profile = load_profile(pool.inner(), &profile_id).await?;
    batch::restart(supervisor.inner(), &profile, &service_ids).await;
    Ok(true)
}

// ── Sondes ───────────────────────────────────────────────────────────────

/// Oublie les lignes gardées pour un service.
///
/// ⚠️ Appelée par le bouton « Effacer » de la vue des logs. Sans elle, l'écran
/// se vide mais le tampon garde tout, et `get_service_logs` (MCP) rend encore
/// des lignes que l'utilisateur croit effacées.
#[tauri::command]
pub async fn dev_logs_clear(
    logs: State<'_, Arc<LogStore>>,
    service_id: String,
) -> Result<(), AppError> {
    logs.clear(&service_id);
    Ok(())
}

#[tauri::command]
pub async fn dev_port_check(port: u16) -> Result<bool, AppError> {
    Ok(net::is_port_available(port).await)
}

#[tauri::command]
pub async fn dev_port_check_batch(
    ports: Vec<PortRequest>,
) -> Result<Vec<PortCheckResult>, AppError> {
    let mut results = Vec::with_capacity(ports.len());
    for request in ports {
        results.push(PortCheckResult {
            service_id: request.service_id,
            port: request.port,
            available: net::is_port_available(request.port).await,
        });
    }
    Ok(results)
}

#[tauri::command]
pub async fn dev_docker_health(
    working_dir: String,
    compose_file: Option<String>,
) -> Result<DockerHealthReport, AppError> {
    let file = compose_file.unwrap_or_else(|| "docker-compose.yml".to_string());
    Ok(docker::compose_health(Path::new(&working_dir), &file).await)
}

#[tauri::command]
pub async fn dev_service_probe(
    pool: State<'_, DbPool>,
    supervisor: State<'_, Arc<Supervisor>>,
    profile_id: String,
) -> Result<Vec<ProbeResult>, AppError> {
    let profile = load_profile(pool.inner(), &profile_id).await?;
    Ok(supervisor.probe(&profile).await)
}

#[tauri::command]
pub async fn dev_process_list(
    supervisor: State<'_, Arc<Supervisor>>,
    profile_id: String,
) -> Result<Vec<ManagedProcessInfo>, AppError> {
    Ok(supervisor.list(&profile_id))
}
