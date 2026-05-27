//! Lynk Dev — backend Rust (entrée bibliothèque Tauri).
//!
//! Le binaire `main.rs` appelle simplement [`run`] qui assemble le pool DB,
//! enregistre les commandes IPC et lance la boucle Tauri.
//!
//! Template de base : fenêtre + tray + thèmes (front) + settings persistés +
//! verrouillage par PIN + auto-update. Les modules métier de Lynk Dev
//! (Git / Dev / DB) viennent se greffer ici.

pub mod commands;
pub mod error;
pub mod store;
pub mod vault;
#[cfg(target_os = "windows")]
mod window_chrome;

pub use error::AppError;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

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
            app.manage(pool);

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
