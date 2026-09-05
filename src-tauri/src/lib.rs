//! Lynk Dev — backend Rust (entrée bibliothèque Tauri).
//!
//! Le binaire `main.rs` appelle simplement [`run`] qui assemble le pool DB,
//! enregistre les commandes IPC et lance la boucle Tauri.
//!
//! Template de base : fenêtre + tray + thèmes (front) + settings persistés +
//! verrouillage par PIN + auto-update. Les modules métier de Lynk Dev
//! (Git / Dev / DB) viennent se greffer ici.

pub mod ai;
pub mod commands;
pub mod dev;
pub mod error;
pub mod git;
pub mod mcp;
pub mod process;
pub mod secrets;
pub mod store;
pub mod vault;
#[cfg(target_os = "windows")]
mod window_chrome;

pub use error::AppError;

use std::sync::Arc;
use std::time::Duration;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};

use dev::types::DevEvent;
use dev::Supervisor;

/// Délai laissé à un port pour se libérer au redémarrage d'un service.
/// L'OS peut encore démonter la socket d'écoute quand on tente de la reprendre.
const PORT_RELEASE_WAIT: Duration = Duration::from_secs(10);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .init();

    // Filet de sécurité : tout panic non rattrapé affiche un message natif au
    // lieu de fermer l'app en silence (windows_subsystem = "windows" masque la
    // console en release, donc un panic = fenêtre qui disparaît sans rien dire).
    std::panic::set_hook(Box::new(|info| {
        let location = info
            .location()
            .map(|l| format!("\n({}:{})", l.file(), l.line()))
            .unwrap_or_default();
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "panique inconnue".into());
        tracing::error!("panic: {msg}{location}");
        native_dialog(
            "Lynk Dev — erreur inattendue",
            &format!(
                "Une erreur est survenue et l'application doit se fermer.\n\nDétail technique :\n{msg}{location}"
            ),
            false,
        );
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("resolve app data dir");
            let db_path = store::default_db_path(&app_data_dir);

            // Init du pool avant le démarrage de Tauri. En cas d'échec, on
            // propose une réinitialisation et on affiche un message clair
            // plutôt que de paniquer silencieusement.
            let pool = init_pool_or_recover(&db_path);
            app.manage(pool.clone());

            // Reprise d'une clé IA restée en clair dans `settings` : elle part
            // dans le trousseau du système, et la ligne est vidée.
            {
                let pool = pool.clone();
                tauri::async_runtime::spawn(async move {
                    commands::ai::migrate_legacy_key(&pool).await;
                });
            }

            // Le superviseur du Dev Manager vit aussi longtemps que l'app, et
            // reste ignorant de Tauri : on se contente de relayer son flux
            // d'événements vers la fenêtre.
            let supervisor = Supervisor::new(PORT_RELEASE_WAIT);
            spawn_dev_event_bridge(app.handle(), &supervisor);

            // Deuxième abonné du même flux : un tampon circulaire, pour les
            // lecteurs qui arrivent après coup — le serveur MCP demande « les
            // 100 dernières lignes », question à laquelle un canal de diffusion
            // ne sait pas répondre.
            let logs = dev::logs::LogStore::new();
            tauri::async_runtime::spawn(logs.drain(supervisor.subscribe()));

            // Le serveur MCP : une façade de plus sur le **même** superviseur.
            let journal = mcp::Journal::new();
            let mcp_server = mcp::McpServer::new(
                mcp::ToolContext {
                    pool: pool.clone(),
                    supervisor: Arc::clone(&supervisor),
                    logs: Arc::clone(&logs),
                },
                Arc::clone(&journal),
                app.package_info().version.to_string(),
            );
            spawn_mcp_journal_bridge(app.handle(), &journal);
            {
                let pool = pool.clone();
                let server = Arc::clone(&mcp_server);
                tauri::async_runtime::spawn(async move {
                    commands::mcp::start_if_enabled(&pool, &server).await;
                });
            }

            app.manage(supervisor);
            app.manage(logs);
            app.manage(mcp_server);

            // Icône de zone de notification (tray). Non bloquant si ça échoue.
            if let Err(e) = build_tray(app.handle()) {
                tracing::warn!("tray setup failed: {e}");
            }

            #[cfg(target_os = "windows")]
            if let Some(window) = app.get_webview_window("main") {
                window_chrome::style_titlebar(&window);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_all_settings,
            commands::settings::set_setting,
            commands::vault::vault_has_pin,
            commands::vault::vault_verify_pin,
            commands::vault::vault_set_pin,
            commands::vault::vault_change_pin,
            commands::vault::vault_disable_pin,
            commands::dev::dev_profile_list,
            commands::dev::dev_profile_save,
            commands::dev::dev_profile_delete,
            commands::dev::dev_scan,
            commands::dev::dev_detect,
            commands::dev::dev_service_start,
            commands::dev::dev_service_stop,
            commands::dev::dev_service_restart,
            commands::dev::dev_service_build,
            commands::dev::dev_service_start_batch,
            commands::dev::dev_service_stop_batch,
            commands::dev::dev_service_restart_batch,
            commands::dev::dev_logs_clear,
            commands::dev::dev_port_check,
            commands::dev::dev_port_check_batch,
            commands::dev::dev_docker_health,
            commands::dev::dev_service_probe,
            commands::dev::dev_process_list,
            commands::git::git_profile_list,
            commands::git::git_profile_save,
            commands::git::git_profile_delete,
            commands::git::git_scan_repos,
            commands::git::git_status,
            commands::git::git_branches,
            commands::git::git_log,
            commands::git::git_stash_list,
            commands::git::git_repo_config,
            commands::git::git_checkout,
            commands::git::git_create_branch,
            commands::git::git_delete_branch,
            commands::git::git_fetch,
            commands::git::git_pull,
            commands::git::git_push,
            commands::git::git_stage,
            commands::git::git_unstage,
            commands::git::git_stage_all,
            commands::git::git_discard_changes,
            commands::git::git_discard_staged,
            commands::git::git_commit,
            commands::git::git_diff,
            commands::git::git_show_file,
            commands::git::git_file_content,
            commands::git::git_merge,
            commands::git::git_merge_abort,
            commands::git::git_resolve_conflict,
            commands::git::git_stash_save,
            commands::git::git_stash_pop,
            commands::git::git_stash_drop,
            commands::git::git_set_config,
            commands::git::git_unset_config,
            commands::git::git_add_remote,
            commands::git::git_remove_remote,
            commands::git::git_set_remote_url,
            commands::git::git_rename_remote,
            commands::git::git_set_branch_upstream,
            commands::git::git_unset_branch_upstream,
            commands::git::git_open_in_terminal,
            commands::ai::ai_config_get,
            commands::ai::ai_config_set,
            commands::ai::ai_list_models,
            commands::ai::ai_commit_message,
            commands::ai::ai_explain_diff,
            commands::ai::ai_summarize_logs,
            commands::mcp::mcp_status,
            commands::mcp::mcp_set_enabled,
            commands::mcp::mcp_set_port,
            commands::mcp::mcp_token,
            commands::mcp::mcp_regenerate_token,
            commands::mcp::mcp_calls,
            commands::mcp::mcp_clear_calls,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // À la fermeture, couper tout ce qu'on supervise : sans ça les
            // services survivent à la fenêtre et gardent leurs ports, et le
            // prochain démarrage échoue en « port déjà utilisé ».
            if matches!(event, tauri::RunEvent::Exit) {
                // Le serveur MCP d'abord : il ne doit plus accepter d'ordre
                // pendant qu'on démonte ce qu'il pilote.
                if let Some(server) = app_handle.try_state::<Arc<mcp::McpServer>>() {
                    let server = Arc::clone(server.inner());
                    tauri::async_runtime::block_on(async move { server.stop().await });
                }
                if let Some(supervisor) = app_handle.try_state::<Arc<Supervisor>>() {
                    let supervisor = Arc::clone(supervisor.inner());
                    tauri::async_runtime::block_on(async move { supervisor.stop_all().await });
                }
            }
        });
}

/// Relaie le flux du superviseur vers les événements de la fenêtre.
///
/// ⚠️ Un retard (`Lagged`) ne doit **pas** interrompre la boucle : un service
/// bavard au démarrage ferait alors taire tous les logs jusqu'au redémarrage de
/// l'application, sans le moindre message d'erreur.
fn spawn_dev_event_bridge(app: &tauri::AppHandle, supervisor: &Arc<Supervisor>) {
    let mut events = supervisor.subscribe();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            match events.recv().await {
                Ok(DevEvent::Log(payload)) => {
                    let _ = app.emit("dev:service:log", payload);
                }
                Ok(DevEvent::Status(payload)) => {
                    let _ = app.emit("dev:service:status", payload);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(missed)) => {
                    tracing::warn!("{missed} evenements Dev Manager perdus (consommateur lent)");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

/// Relaie le journal du serveur MCP vers la fenêtre.
///
/// Même précaution que pour le flux du Dev Manager : un retard ne rompt pas la
/// boucle, sinon le journal se tait pour le reste de la session.
fn spawn_mcp_journal_bridge(app: &tauri::AppHandle, journal: &Arc<mcp::Journal>) {
    let mut calls = journal.subscribe();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            match calls.recv().await {
                Ok(record) => {
                    let _ = app.emit("mcp:call", record);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(missed)) => {
                    tracing::warn!("{missed} appels MCP non affichés (consommateur lent)");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

/// Boîte de dialogue native bloquante. Sur Windows : `MessageBoxW`. Ailleurs :
/// log sur stderr (les autres OS ne masquent pas la console de la même façon).
/// Retourne `true` si l'utilisateur clique « Oui » (uniquement en mode yes/no).
#[cfg(target_os = "windows")]
fn native_dialog(title: &str, body: &str, yes_no: bool) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, IDYES, MB_ICONERROR, MB_ICONWARNING, MB_OK, MB_YESNO,
    };
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }
    let title_w = wide(title);
    let body_w = wide(body);
    let flags = if yes_no {
        MB_YESNO | MB_ICONWARNING
    } else {
        MB_OK | MB_ICONERROR
    };
    let ret = unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            body_w.as_ptr(),
            title_w.as_ptr(),
            flags,
        )
    };
    yes_no && ret == IDYES
}

#[cfg(not(target_os = "windows"))]
fn native_dialog(title: &str, body: &str, _yes_no: bool) -> bool {
    eprintln!("[{title}] {body}");
    false
}

/// Met de côté les fichiers de base de données (`.sqlite`, `-wal`, `-shm`).
fn backup_db_files(db_path: &std::path::Path) {
    let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    for suffix in ["", "-wal", "-shm"] {
        let src = std::path::PathBuf::from(format!("{}{}", db_path.display(), suffix));
        if src.exists() {
            let dst =
                std::path::PathBuf::from(format!("{}.bak-{}{}", db_path.display(), stamp, suffix));
            if let Err(e) = std::fs::rename(&src, &dst) {
                tracing::warn!("backup {} failed: {e}", src.display());
            }
        }
    }
}

/// Ouvre la base ; en cas d'échec, propose une réinitialisation (sauvegarde de
/// l'ancienne base) et réessaie. Jamais de fermeture silencieuse.
fn init_pool_or_recover(db_path: &std::path::Path) -> store::DbPool {
    match tauri::async_runtime::block_on(store::init_pool(db_path)) {
        Ok(pool) => pool,
        Err(err) => {
            tracing::error!("db init failed: {err:#}");
            let reset = native_dialog(
                "Lynk Dev — base de données",
                &format!(
                    "Impossible d'ouvrir la base de données locale.\n\nDétail : {err}\n\nVoulez-vous réinitialiser les données locales ?"
                ),
                true,
            );
            if !reset {
                native_dialog(
                    "Lynk Dev",
                    "Démarrage annulé. L'application va se fermer.",
                    false,
                );
                std::process::exit(1);
            }
            backup_db_files(db_path);
            match tauri::async_runtime::block_on(store::init_pool(db_path)) {
                Ok(pool) => pool,
                Err(err2) => {
                    tracing::error!("db re-init after reset failed: {err2:#}");
                    native_dialog(
                        "Lynk Dev",
                        &format!(
                            "La réinitialisation a échoué :\n{err2}\n\nL'application va se fermer."
                        ),
                        false,
                    );
                    std::process::exit(1);
                }
            }
        }
    }
}

/// Affiche et met au premier plan la fenêtre principale (depuis le tray).
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Crée l'icône de zone de notification (tray) avec un menu Ouvrir / Quitter.
/// Clic gauche → ouvre la fenêtre ; clic droit → menu.
fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "tray-show", "Ouvrir Lynk Dev", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "tray-quit", "Quitter", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    TrayIconBuilder::with_id("lynk-dev-tray")
        .icon(tauri::include_image!("icons/128x128.png"))
        .tooltip("Lynk Dev")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "tray-show" => show_main_window(app),
            "tray-quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}
