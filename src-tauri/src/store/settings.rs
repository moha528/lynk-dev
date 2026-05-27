//! DAO for the `settings` key/value table.
//!
//! Values are stored as JSON strings (`serde_json::Value` on disk), giving us
//! a stable schema for everything from booleans to nested objects without ever
//! needing a new migration.

use anyhow::{Context, Result};
use serde_json::Value;

use super::DbPool;

/// Get a setting by key. Returns `None` when the key is absent.
pub async fn get(pool: &DbPool, key: &str) -> Result<Option<Value>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?1")
        .bind(key)
        .fetch_optional(pool)
        .await
        .context("select setting")?;
    match row {
        Some((raw,)) => Ok(Some(
            serde_json::from_str(&raw).with_context(|| format!("decode setting {key}"))?,
        )),
        None => Ok(None),
    }
}

/// List every setting as a JSON object (`{ key: value, ... }`). Convenient for
/// the frontend to hydrate its state at startup with a single IPC call.
pub async fn all(pool: &DbPool) -> Result<serde_json::Map<String, Value>> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings")
        .fetch_all(pool)
        .await
        .context("select all settings")?;
    let mut out = serde_json::Map::with_capacity(rows.len());
    for (k, raw) in rows {
        let v: Value = serde_json::from_str(&raw).with_context(|| format!("decode setting {k}"))?;
        out.insert(k, v);
    }
    Ok(out)
}

/// Upsert a setting.
pub async fn set(pool: &DbPool, key: &str, value: &Value) -> Result<()> {
    let raw = serde_json::to_string(value).context("encode setting")?;
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at)
         VALUES (?1, ?2, datetime('now'))
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
    )
    .bind(key)
    .bind(&raw)
    .execute(pool)
    .await
    .context("upsert setting")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::init_pool;
    use serde_json::json;

    async fn fresh_pool() -> DbPool {
        let tmp = tempfile::Builder::new()
            .suffix(".sqlite")
            .tempfile()
            .expect("tmp");
        let path = tmp.path().to_path_buf();
        drop(tmp);
        init_pool(&path).await.expect("pool")
    }

    #[tokio::test]
    async fn round_trip_primitive() {
        let pool = fresh_pool().await;
        assert!(get(&pool, "sidebar_width").await.unwrap().is_none());

        set(&pool, "sidebar_width", &json!(280)).await.unwrap();
        assert_eq!(get(&pool, "sidebar_width").await.unwrap(), Some(json!(280)));

        // Upsert overwrites.
        set(&pool, "sidebar_width", &json!(320)).await.unwrap();
        assert_eq!(get(&pool, "sidebar_width").await.unwrap(), Some(json!(320)));
    }

    #[tokio::test]
    async fn round_trip_object() {
        let pool = fresh_pool().await;
        let value = json!({ "width": 1280, "height": 800 });
        set(&pool, "window", &value).await.unwrap();
        assert_eq!(get(&pool, "window").await.unwrap(), Some(value));
    }

    #[tokio::test]
    async fn all_returns_every_entry() {
        let pool = fresh_pool().await;
        set(&pool, "a", &json!(1)).await.unwrap();
        set(&pool, "b", &json!("two")).await.unwrap();
        let map = all(&pool).await.unwrap();
        assert_eq!(map.get("a"), Some(&json!(1)));
        assert_eq!(map.get("b"), Some(&json!("two")));
    }
}
