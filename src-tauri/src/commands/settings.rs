//! Tauri commands exposing the key/value [`crate::store::settings`] table.

use serde_json::{Map, Value};
use tauri::State;

use crate::store::{settings as dao, DbPool};
use crate::AppError;

#[tauri::command]
pub async fn get_all_settings(pool: State<'_, DbPool>) -> Result<Map<String, Value>, AppError> {
    Ok(dao::all(pool.inner()).await?)
}

#[tauri::command]
pub async fn set_setting(
    pool: State<'_, DbPool>,
    key: String,
    value: Value,
) -> Result<(), AppError> {
    dao::set(pool.inner(), &key, &value).await?;
    Ok(())
}
