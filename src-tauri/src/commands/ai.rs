//! Pont IPC de l'assistance par modèle.
//!
//! ⚠️ **La clé n'est jamais rendue au front.** L'écran de réglages sait
//! seulement s'il y en a une ; il peut la remplacer ou l'effacer, pas la lire.
//! Un champ qui réaffiche un secret finit toujours par le montrer sur une
//! capture d'écran.
//!
//! ⚠️ **La clé vit dans le trousseau du système**, pas dans la base locale
//! (cf. [`crate::secrets`]). Le modèle choisi, lui, n'est pas un secret et reste
//! dans `settings`.

use std::path::Path;

use serde::Serialize;
use serde_json::json;
use tauri::State;

use crate::ai::openrouter::{self, Completion, ModelInfo};
use crate::ai::prompts;
use crate::git::repo;
use crate::secrets;
use crate::store::{settings as dao, DbPool};
use crate::AppError;

/// Ancien emplacement de la clé — en clair dans `settings`. Conservé pour la
/// seule reprise ([`migrate_legacy_key`]), jamais écrit à nouveau.
const LEGACY_KEY_API: &str = "ai_api_key";
const KEY_MODEL: &str = "ai_model";

/// Réponses courtes : un message de commit tient en quelques lignes, une
/// explication en cinq. Plafonner, c'est plafonner la facture.
const MAX_TOKENS_COMMIT: u32 = 400;
const MAX_TOKENS_EXPLAIN: u32 = 500;
const MAX_TOKENS_LOGS: u32 = 700;

/// Ce que l'écran de réglages a le droit de savoir.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConfig {
    /// Une clé est enregistrée — sa valeur, elle, ne sort jamais d'ici.
    pub api_key_set: bool,
    pub model: Option<String>,
    /// Renseigné quand le trousseau du système est inutilisable. L'écran le
    /// montre tel quel : sans trousseau, la fonction est hors service, et le
    /// dire vaut mieux qu'un « échec » sans cause.
    pub keychain_error: Option<String>,
}

async fn read_string(pool: &DbPool, key: &str) -> Result<Option<String>, AppError> {
    Ok(dao::get(pool, key)
        .await?
        .and_then(|value| value.as_str().map(str::to_string))
        .filter(|value| !value.trim().is_empty()))
}

async fn read_api_key() -> Result<Option<String>, AppError> {
    Ok(secrets::get(secrets::OPENROUTER_KEY)
        .await?
        .filter(|value| !value.trim().is_empty()))
}

/// Clé et modèle, ou une erreur explicite si l'un des deux manque.
async fn credentials(pool: &DbPool) -> Result<(String, String), AppError> {
    let Some(api_key) = read_api_key().await? else {
        return Err(AppError(anyhow::anyhow!(
            "aucune clé OpenRouter enregistrée — Réglages > IA"
        )));
    };
    let Some(model) = read_string(pool, KEY_MODEL).await? else {
        return Err(AppError(anyhow::anyhow!(
            "aucun modèle choisi — Réglages > IA"
        )));
    };
    Ok((api_key, model))
}

/// Déplace une clé restée en clair dans `settings` vers le trousseau, puis
/// efface la ligne.
///
/// Appelée une fois au démarrage. ⚠️ La ligne n'est effacée **qu'après** une
/// écriture réussie : sur une machine sans trousseau, perdre la clé en plus de
/// ne pas pouvoir la protéger serait la pire des deux issues.
pub async fn migrate_legacy_key(pool: &DbPool) {
    let legacy = match read_string(pool, LEGACY_KEY_API).await {
        Ok(Some(value)) => value,
        Ok(None) => return,
        Err(err) => {
            tracing::warn!("reprise de la clé IA : lecture impossible ({err})");
            return;
        }
    };

    match secrets::set(secrets::OPENROUTER_KEY, legacy.trim()).await {
        Ok(()) => {
            if let Err(err) = dao::set(pool, LEGACY_KEY_API, &json!("")).await {
                tracing::warn!("clé IA déplacée mais ligne en clair non vidée : {err}");
            } else {
                tracing::info!("clé IA déplacée de la base locale vers le trousseau");
            }
        }
        Err(err) => tracing::warn!("clé IA laissée en base : trousseau indisponible ({err:#})"),
    }
}

// ── Réglages ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn ai_config_get(pool: State<'_, DbPool>) -> Result<AiConfig, AppError> {
    let keychain_error = secrets::store_error();
    Ok(AiConfig {
        // Sans trousseau, inutile d'interroger : la réponse serait une erreur,
        // et l'écran a déjà de quoi expliquer pourquoi.
        api_key_set: keychain_error.is_none() && read_api_key().await?.is_some(),
        model: read_string(pool.inner(), KEY_MODEL).await?,
        keychain_error,
    })
}

/// Enregistre la clé et/ou le modèle. `None` laisse la valeur en place ; une
/// chaîne vide efface.
#[tauri::command]
pub async fn ai_config_set(
    pool: State<'_, DbPool>,
    api_key: Option<String>,
    model: Option<String>,
) -> Result<(), AppError> {
    if let Some(api_key) = api_key {
        let api_key = api_key.trim();
        if api_key.is_empty() {
            secrets::delete(secrets::OPENROUTER_KEY).await?;
        } else {
            secrets::set(secrets::OPENROUTER_KEY, api_key).await?;
        }
    }
    if let Some(model) = model {
        dao::set(pool.inner(), KEY_MODEL, &json!(model.trim())).await?;
    }
    Ok(())
}

/// Catalogue en direct, du moins cher au plus cher.
///
/// Accepte une clé passée en argument : l'écran de réglages doit pouvoir
/// éprouver une clé **avant** de l'enregistrer.
#[tauri::command]
pub async fn ai_list_models(api_key: Option<String>) -> Result<Vec<ModelInfo>, AppError> {
    let key = match api_key.filter(|value| !value.trim().is_empty()) {
        Some(provided) => provided,
        None => read_api_key()
            .await?
            .ok_or_else(|| AppError(anyhow::anyhow!("aucune clé OpenRouter enregistrée")))?,
    };
    Ok(openrouter::list_models(&key).await?)
}

// ── Les trois usages ─────────────────────────────────────────────────────

/// Rédige un message de commit à partir de ce qui est **déjà indexé**.
///
/// L'index fait foi, pas la sélection de l'écran : c'est exactement ce qui sera
/// validé.
#[tauri::command]
pub async fn ai_commit_message(
    pool: State<'_, DbPool>,
    repo_path: String,
) -> Result<Completion, AppError> {
    let (api_key, model) = credentials(pool.inner()).await?;
    let diff = repo::staged_diff(Path::new(&repo_path)).await?;
    if diff.trim().is_empty() {
        return Err(AppError(anyhow::anyhow!(
            "rien dans l'index — indexez au moins un fichier"
        )));
    }
    Ok(openrouter::complete(
        &api_key,
        &model,
        prompts::COMMIT_SYSTEM,
        &prompts::commit_user(&diff),
        MAX_TOKENS_COMMIT,
    )
    .await?)
}

#[tauri::command]
pub async fn ai_explain_diff(
    pool: State<'_, DbPool>,
    diff: String,
) -> Result<Completion, AppError> {
    let (api_key, model) = credentials(pool.inner()).await?;
    if diff.trim().is_empty() {
        return Err(AppError(anyhow::anyhow!("aucun diff à expliquer")));
    }
    Ok(openrouter::complete(
        &api_key,
        &model,
        prompts::EXPLAIN_SYSTEM,
        &prompts::explain_user(&diff),
        MAX_TOKENS_EXPLAIN,
    )
    .await?)
}

#[tauri::command]
pub async fn ai_summarize_logs(
    pool: State<'_, DbPool>,
    logs: String,
) -> Result<Completion, AppError> {
    let (api_key, model) = credentials(pool.inner()).await?;
    if logs.trim().is_empty() {
        return Err(AppError(anyhow::anyhow!("aucune ligne à analyser")));
    }
    Ok(openrouter::complete(
        &api_key,
        &model,
        prompts::LOGS_SYSTEM,
        &prompts::logs_user(&logs),
        MAX_TOKENS_LOGS,
    )
    .await?)
}
