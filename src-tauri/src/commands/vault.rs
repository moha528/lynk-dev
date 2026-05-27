//! `vault_*` Tauri commands. Wrap [`crate::vault`] for the front-end
//! settings page + the startup unlock dialog.

use tauri::State;

use crate::store::DbPool;
use crate::vault;
use crate::AppError;

#[tauri::command]
pub async fn vault_has_pin(pool: State<'_, DbPool>) -> Result<bool, AppError> {
    Ok(vault::has_pin(pool.inner()).await?)
}

#[tauri::command]
pub async fn vault_verify_pin(pool: State<'_, DbPool>, pin: String) -> Result<bool, AppError> {
    Ok(vault::verify_pin(pool.inner(), &pin).await?)
}

#[tauri::command]
pub async fn vault_set_pin(pool: State<'_, DbPool>, new_pin: String) -> Result<(), AppError> {
    vault::set_pin(pool.inner(), &new_pin)
        .await
        .map_err(AppError)
}

#[tauri::command]
pub async fn vault_change_pin(
    pool: State<'_, DbPool>,
    current_pin: String,
    new_pin: String,
) -> Result<(), AppError> {
    vault::change_pin(pool.inner(), &current_pin, &new_pin)
        .await
        .map_err(AppError)
}

#[tauri::command]
pub async fn vault_disable_pin(
    pool: State<'_, DbPool>,
    current_pin: String,
) -> Result<(), AppError> {
    vault::disable_pin(pool.inner(), &current_pin)
        .await
        .map_err(AppError)
}
