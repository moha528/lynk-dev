//! DAO des profils du Dev Manager.

use anyhow::{Context, Result};

use crate::dev::types::{DevProfile, ServiceConfig};

use super::DbPool;

/// Reconstruit un profil depuis sa ligne, en décodant la colonne `services`.
///
/// Un JSON de services illisible ne doit pas faire disparaître le profil :
/// on rend la liste vide et on le signale, l'utilisateur reste maître de son
/// écran plutôt que de perdre son profil sans explication.
fn to_profile(row: (String, String, String, String, i64)) -> DevProfile {
    let (id, name, root_path, services, created_at) = row;
    let services: Vec<ServiceConfig> = serde_json::from_str(&services).unwrap_or_else(|err| {
        tracing::warn!("profil {id}: services illisibles ({err}) - liste vidée");
        Vec::new()
    });
    DevProfile {
        id,
        name,
        root_path,
        services,
        created_at,
    }
}

const SELECT: &str = "SELECT id, name, root_path, services, created_at FROM dev_profiles";

pub async fn all(pool: &DbPool) -> Result<Vec<DevProfile>> {
    let rows: Vec<(String, String, String, String, i64)> =
        sqlx::query_as(&format!("{SELECT} ORDER BY created_at"))
            .fetch_all(pool)
            .await
            .context("select dev profiles")?;
    Ok(rows.into_iter().map(to_profile).collect())
}

pub async fn get(pool: &DbPool, id: &str) -> Result<Option<DevProfile>> {
    let row: Option<(String, String, String, String, i64)> =
        sqlx::query_as(&format!("{SELECT} WHERE id = ?1"))
            .bind(id)
            .fetch_optional(pool)
            .await
            .context("select dev profile")?;
    Ok(row.map(to_profile))
}

pub async fn save(pool: &DbPool, profile: &DevProfile) -> Result<()> {
    let services = serde_json::to_string(&profile.services).context("encode services")?;
    sqlx::query(
        "INSERT INTO dev_profiles (id, name, root_path, services, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
             name = excluded.name,
             root_path = excluded.root_path,
             services = excluded.services,
             updated_at = datetime('now')",
    )
    .bind(&profile.id)
    .bind(&profile.name)
    .bind(&profile.root_path)
    .bind(&services)
    .bind(profile.created_at)
    .execute(pool)
    .await
    .context("upsert dev profile")?;
    Ok(())
}

pub async fn delete(pool: &DbPool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM dev_profiles WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .context("delete dev profile")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev::types::ServiceType;
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

    fn profile(id: &str) -> DevProfile {
        DevProfile {
            id: id.into(),
            name: format!("profil {id}"),
            root_path: "C:/work".into(),
            services: vec![ServiceConfig {
                id: "auth".into(),
                name: "olive_auth_service".into(),
                kind: ServiceType::SpringBootMaven,
                working_dir: "C:/work/back/olive_auth_service".into(),
                command: "mvnw.cmd spring-boot:run".into(),
                build_command: None,
                port: Some(8010),
                health_check_url: None,
                group: None,
                depends_on: Some(vec!["postgres".into()]),
                env_vars: None,
                auto_restart: true,
            }],
            created_at: 1_700_000_000_000,
        }
    }

    #[tokio::test]
    async fn saves_and_reads_back_with_its_services() {
        let pool = fresh_pool().await;
        save(&pool, &profile("p1")).await.expect("save");

        let found = get(&pool, "p1").await.expect("get").expect("present");
        assert_eq!(found.name, "profil p1");
        assert_eq!(found.services.len(), 1);
        assert_eq!(found.services[0].port, Some(8010));
        assert!(found.services[0].auto_restart);
        assert_eq!(found.created_at, 1_700_000_000_000);
    }

    #[tokio::test]
    async fn save_twice_updates_instead_of_duplicating() {
        let pool = fresh_pool().await;
        save(&pool, &profile("p1")).await.expect("save");

        let mut updated = profile("p1");
        updated.name = "renomme".into();
        updated.services.clear();
        save(&pool, &updated).await.expect("resave");

        let list = all(&pool).await.expect("all");
        assert_eq!(list.len(), 1, "un seul profil");
        assert_eq!(list[0].name, "renomme");
        assert!(list[0].services.is_empty());
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
    async fn get_on_an_unknown_id_is_none() {
        let pool = fresh_pool().await;
        assert!(get(&pool, "nope").await.expect("get").is_none());
    }

    /// Un JSON corrompu ne doit pas faire disparaître le profil.
    #[tokio::test]
    async fn corrupt_services_json_yields_an_empty_list_not_a_lost_profile() {
        let pool = fresh_pool().await;
        save(&pool, &profile("p1")).await.expect("save");
        sqlx::query("UPDATE dev_profiles SET services = ?1 WHERE id = ?2")
            .bind("{ pas du json")
            .bind("p1")
            .execute(&pool)
            .await
            .expect("corrupt");

        let found = get(&pool, "p1").await.expect("get").expect("toujours la");
        assert_eq!(found.name, "profil p1");
        assert!(found.services.is_empty());
    }
}
