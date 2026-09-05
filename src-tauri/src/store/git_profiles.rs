//! DAO des profils du Git Manager.

use anyhow::{Context, Result};

use crate::git::types::GitProfile;

use super::DbPool;

/// Un JSON de chemins illisible vide la liste mais **ne fait pas disparaître le
/// profil** : l'utilisateur garde son écran et peut re-analyser sa racine.
fn to_profile(row: (String, String, String, String, i64)) -> GitProfile {
    let (id, name, root_path, repo_paths, created_at) = row;
    let repo_paths: Vec<String> = serde_json::from_str(&repo_paths).unwrap_or_else(|err| {
        tracing::warn!("profil git {id}: chemins illisibles ({err}) - liste vidée");
        Vec::new()
    });
    GitProfile {
        id,
        name,
        root_path,
        repo_paths,
        created_at,
    }
}

const SELECT: &str = "SELECT id, name, root_path, repo_paths, created_at FROM git_profiles";

pub async fn all(pool: &DbPool) -> Result<Vec<GitProfile>> {
    let rows: Vec<(String, String, String, String, i64)> =
        sqlx::query_as(&format!("{SELECT} ORDER BY created_at"))
            .fetch_all(pool)
            .await
            .context("select git profiles")?;
    Ok(rows.into_iter().map(to_profile).collect())
}

pub async fn get(pool: &DbPool, id: &str) -> Result<Option<GitProfile>> {
    let row: Option<(String, String, String, String, i64)> =
        sqlx::query_as(&format!("{SELECT} WHERE id = ?1"))
            .bind(id)
            .fetch_optional(pool)
            .await
            .context("select git profile")?;
    Ok(row.map(to_profile))
}

pub async fn save(pool: &DbPool, profile: &GitProfile) -> Result<()> {
    let repo_paths = serde_json::to_string(&profile.repo_paths).context("encode repo paths")?;
    sqlx::query(
        "INSERT INTO git_profiles (id, name, root_path, repo_paths, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
             name = excluded.name,
             root_path = excluded.root_path,
             repo_paths = excluded.repo_paths,
             updated_at = datetime('now')",
    )
    .bind(&profile.id)
    .bind(&profile.name)
    .bind(&profile.root_path)
    .bind(&repo_paths)
    .bind(profile.created_at)
    .execute(pool)
    .await
    .context("upsert git profile")?;
    Ok(())
}

pub async fn delete(pool: &DbPool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM git_profiles WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .context("delete git profile")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::init_pool;

    async fn fresh_pool() -> DbPool {
        let tmp = tempfile::Builder::new()
            .suffix(".sqlite")
            .tempfile()
            .expect("tmp");
        let path = tmp.path().to_path_buf();
        drop(tmp);
        init_pool(&path).await.expect("pool")
    }

    fn profile(id: &str) -> GitProfile {
        GitProfile {
            id: id.into(),
            name: format!("profil {id}"),
            root_path: "C:/work/zeitune".into(),
            repo_paths: vec![
                "C:/work/zeitune/back/olive_core".into(),
                "C:/work/zeitune/front/olive_front".into(),
            ],
            created_at: 1_700_000_000_000,
        }
    }

    #[tokio::test]
    async fn saves_and_reads_back_its_repo_paths() {
        let pool = fresh_pool().await;
        save(&pool, &profile("p1")).await.expect("save");

        let found = get(&pool, "p1").await.expect("get").expect("present");
        assert_eq!(found.repo_paths.len(), 2);
        assert!(found.repo_paths[0].ends_with("olive_core"));
    }

    #[tokio::test]
    async fn save_twice_updates_instead_of_duplicating() {
        let pool = fresh_pool().await;
        save(&pool, &profile("p1")).await.expect("save");
        let mut updated = profile("p1");
        updated.name = "renomme".into();
        updated.repo_paths.clear();
        save(&pool, &updated).await.expect("resave");

        let list = all(&pool).await.expect("all");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "renomme");
        assert!(list[0].repo_paths.is_empty());
    }

    #[tokio::test]
    async fn delete_removes_only_the_target() {
        let pool = fresh_pool().await;
        save(&pool, &profile("p1")).await.expect("save");
        save(&pool, &profile("p2")).await.expect("save");
        delete(&pool, "p1").await.expect("delete");

        let list = all(&pool).await.expect("all");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "p2");
    }

    #[tokio::test]
    async fn corrupt_paths_json_yields_an_empty_list_not_a_lost_profile() {
        let pool = fresh_pool().await;
        save(&pool, &profile("p1")).await.expect("save");
        sqlx::query("UPDATE git_profiles SET repo_paths = ?1 WHERE id = ?2")
            .bind("[[[")
            .bind("p1")
            .execute(&pool)
            .await
            .expect("corrupt");

        let found = get(&pool, "p1").await.expect("get").expect("toujours la");
        assert_eq!(found.name, "profil p1");
        assert!(found.repo_paths.is_empty());
    }
}
