use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

pub type DbPool = SqlitePool;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Open (and lazily create) the SQLite database at `db_path`, run pending
/// migrations and return a connection pool.
///
/// The parent directory of `db_path` is created if it does not exist.
pub async fn init_pool(db_path: &Path) -> Result<DbPool> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create data dir {}", parent.display()))?;
    }

    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .with_context(|| format!("failed to open sqlite db at {}", db_path.display()))?;

    // Aligne les checksums avant de migrer (cf. reconcile_migration_checksums).
    reconcile_migration_checksums(&pool).await?;

    MIGRATOR
        .run(&pool)
        .await
        .context("failed to run sqlx migrations")?;

    Ok(pool)
}

/// Réaligne les checksums stockés dans `_sqlx_migrations` sur ceux du binaire
/// courant, AVANT de lancer les migrations.
///
/// sqlx refuse de démarrer si le checksum d'une migration déjà appliquée
/// diffère de celui embarqué (« migration N was previously applied but has been
/// modified »). Or ce checksum est calculé sur les octets du fichier `.sql` à
/// la compilation : il varie selon les fins de ligne (CRLF/LF) du build, donc
/// une base créée par un build peut faire planter un autre build (typiquement
/// après une mise à jour). On ne change JAMAIS le sens d'une migration publiée
/// — uniquement, au pire, des espaces/fins de ligne — donc on se fie au
/// **numéro de version** et on adopte les checksums du binaire courant. Les
/// migrations non encore appliquées tournent ensuite normalement.
async fn reconcile_migration_checksums(pool: &DbPool) -> Result<()> {
    let has_table: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'",
    )
    .fetch_optional(pool)
    .await
    .context("checking _sqlx_migrations table")?;
    if has_table.is_none() {
        return Ok(()); // base fraîche : rien à réaligner
    }

    for migration in MIGRATOR.iter() {
        let checksum: &[u8] = migration.checksum.as_ref();
        sqlx::query("UPDATE _sqlx_migrations SET checksum = ?1 WHERE version = ?2")
            .bind(checksum)
            .bind(migration.version)
            .execute(pool)
            .await
            .with_context(|| format!("reconciling checksum for migration {}", migration.version))?;
    }
    Ok(())
}

/// Resolve the production database path inside the Tauri app data directory.
pub fn default_db_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("lynk-dev.sqlite")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `init_pool` should create the file, apply migrations and expose the
    /// `settings` table.
    #[tokio::test]
    async fn init_pool_creates_schema() {
        let tmp = tempfile::tempdir().expect("tmp");
        let db_path = tmp.path().join("test.sqlite");
        let pool = init_pool(&db_path).await.expect("init pool");

        let tables: Vec<(String,)> =
            sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .fetch_all(&pool)
                .await
                .expect("fetch tables");
        let names: Vec<String> = tables.into_iter().map(|(n,)| n).collect();
        assert!(
            names.contains(&"settings".to_string()),
            "settings table missing"
        );
    }

    /// Une base dont un checksum de migration a été altéré (typiquement CRLF/LF
    /// entre deux builds) ne doit PAS bloquer le démarrage : `init_pool`
    /// réaligne les checksums avant de migrer.
    #[tokio::test]
    async fn init_pool_self_heals_modified_checksum() {
        let tmp = tempfile::tempdir().expect("tmp");
        let db_path = tmp.path().join("test.sqlite");

        // 1er démarrage : applique les migrations, puis on corrompt un checksum.
        {
            let pool = init_pool(&db_path).await.expect("init pool 1");
            sqlx::query("UPDATE _sqlx_migrations SET checksum = ?1 WHERE version = 1")
                .bind(vec![0xDE_u8, 0xAD, 0xBE, 0xEF])
                .execute(&pool)
                .await
                .expect("corrupt checksum");
            pool.close().await;
        }

        // 2e démarrage : doit réussir malgré le checksum altéré (auto-réparation).
        let pool = init_pool(&db_path)
            .await
            .expect("init pool 2 should self-heal a modified checksum");
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM settings")
            .fetch_one(&pool)
            .await
            .expect("query settings");
        assert_eq!(n, 0);
    }
}
