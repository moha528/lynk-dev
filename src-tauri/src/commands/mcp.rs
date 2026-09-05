//! Pont IPC du serveur MCP.
//!
//! ⚠️ **Aucune logique métier ici** — même règle que `commands/dev.rs`. Ces
//! commandes lisent les réglages, démarrent ou arrêtent l'écoute, et rendent
//! l'état. Les outils, eux, s'adressent au superviseur sans passer par Tauri.
//!
//! ⚠️ Le jeton MCP, contrairement à la clé OpenRouter, **est relisible** : sans
//! ça l'utilisateur ne peut pas le coller dans la configuration de son client
//! IA. Il vit malgré tout dans le trousseau, pas dans la base en clair.

use std::sync::Arc;

use serde::Serialize;
use serde_json::json;
use tauri::State;

use crate::mcp::{self, CallRecord, McpServer};
use crate::secrets;
use crate::store::{settings as dao, DbPool};
use crate::AppError;

const KEY_ENABLED: &str = "mcp_enabled";
const KEY_PORT: &str = "mcp_port";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    /// Ce que l'utilisateur a demandé.
    pub enabled: bool,
    /// Ce qui est vrai. Les deux divergent quand le port est déjà pris.
    pub running: bool,
    /// Port d'écoute effectif s'il tourne, sinon celui qui sera tenté.
    pub port: u16,
    pub url: String,
    /// Renseigné quand le trousseau est inutilisable : sans lui, pas de jeton,
    /// donc pas de serveur.
    pub keychain_error: Option<String>,
}

async fn read_port(pool: &DbPool) -> Result<u16, AppError> {
    Ok(dao::get(pool, KEY_PORT)
        .await?
        .and_then(|value| value.as_u64())
        .and_then(|value| u16::try_from(value).ok())
        .filter(|port| *port > 0)
        .unwrap_or(mcp::DEFAULT_PORT))
}

async fn read_enabled(pool: &DbPool) -> Result<bool, AppError> {
    Ok(dao::get(pool, KEY_ENABLED)
        .await?
        .and_then(|value| value.as_bool())
        .unwrap_or(false))
}

/// Le jeton existant, ou un nouveau si c'est la première fois.
async fn ensure_token() -> Result<String, AppError> {
    if let Some(token) = secrets::get(secrets::MCP_TOKEN).await? {
        if !token.trim().is_empty() {
            return Ok(token);
        }
    }
    let token = mcp::generate_token();
    secrets::set(secrets::MCP_TOKEN, &token).await?;
    Ok(token)
}

/// Démarre l'écoute si l'utilisateur l'avait laissée active.
///
/// Appelée une fois au lancement. ⚠️ Un échec ne doit **pas** empêcher
/// l'application de s'ouvrir : le port peut avoir été pris entre deux sessions,
/// et l'écran de réglages dira alors « demandé, mais pas en écoute ».
pub async fn start_if_enabled(pool: &DbPool, server: &Arc<McpServer>) {
    match read_enabled(pool).await {
        Ok(false) | Err(_) => return,
        Ok(true) => {}
    }
    let port = match read_port(pool).await {
        Ok(port) => port,
        Err(err) => {
            tracing::warn!("serveur MCP : port illisible ({err})");
            return;
        }
    };
    let token = match ensure_token().await {
        Ok(token) => token,
        Err(err) => {
            tracing::warn!("serveur MCP non démarré : jeton indisponible ({err})");
            return;
        }
    };
    if let Err(err) = server.start(port, token).await {
        tracing::warn!("serveur MCP non démarré : {err:#}");
    }
}

// ── Réglages ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn mcp_status(
    pool: State<'_, DbPool>,
    server: State<'_, Arc<McpServer>>,
) -> Result<McpStatus, AppError> {
    let running_port = server.port().await;
    let port = running_port.unwrap_or(read_port(pool.inner()).await?);
    Ok(McpStatus {
        enabled: read_enabled(pool.inner()).await?,
        running: running_port.is_some(),
        port,
        url: format!("http://127.0.0.1:{port}{}", mcp::server::PATH),
        keychain_error: secrets::store_error(),
    })
}

/// Active ou coupe l'écoute, et retient le choix pour la prochaine session.
///
/// ⚠️ Le réglage n'est enregistré **qu'après** un démarrage réussi : retenir
/// « activé » alors que le port est pris ferait échouer chaque lancement en
/// silence.
#[tauri::command]
pub async fn mcp_set_enabled(
    pool: State<'_, DbPool>,
    server: State<'_, Arc<McpServer>>,
    enabled: bool,
) -> Result<McpStatus, AppError> {
    if enabled {
        let port = read_port(pool.inner()).await?;
        let token = ensure_token().await?;
        server.start(port, token).await?;
    } else {
        server.stop().await;
    }
    dao::set(pool.inner(), KEY_ENABLED, &json!(enabled)).await?;
    mcp_status(pool, server).await
}

/// Change le port. Si le serveur tourne, il redémarre sur le nouveau.
#[tauri::command]
pub async fn mcp_set_port(
    pool: State<'_, DbPool>,
    server: State<'_, Arc<McpServer>>,
    port: u16,
) -> Result<McpStatus, AppError> {
    if port == 0 {
        return Err(AppError(anyhow::anyhow!("port invalide")));
    }
    let was_running = server.port().await.is_some();
    if was_running {
        // On démarre le nouveau **avant** d'enregistrer : si le port est déjà
        // pris, l'ancien réglage reste, et l'erreur remonte à l'écran.
        server.start(port, ensure_token().await?).await?;
    }
    dao::set(pool.inner(), KEY_PORT, &json!(port)).await?;
    mcp_status(pool, server).await
}

/// Le jeton, en clair — il est fait pour être copié dans la configuration du
/// client IA.
#[tauri::command]
pub async fn mcp_token() -> Result<Option<String>, AppError> {
    Ok(secrets::get(secrets::MCP_TOKEN).await?)
}

/// Nouveau jeton. L'ancien cesse d'être accepté immédiatement : le serveur, s'il
/// tourne, redémarre avec le nouveau.
#[tauri::command]
pub async fn mcp_regenerate_token(
    pool: State<'_, DbPool>,
    server: State<'_, Arc<McpServer>>,
) -> Result<String, AppError> {
    let token = mcp::generate_token();
    secrets::set(secrets::MCP_TOKEN, &token).await?;
    if server.port().await.is_some() {
        let port = read_port(pool.inner()).await?;
        server.start(port, token.clone()).await?;
    }
    Ok(token)
}

// ── Journal ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn mcp_calls(server: State<'_, Arc<McpServer>>) -> Result<Vec<CallRecord>, AppError> {
    Ok(server.journal().recent())
}

#[tauri::command]
pub async fn mcp_clear_calls(server: State<'_, Arc<McpServer>>) -> Result<(), AppError> {
    server.journal().clear();
    Ok(())
}
