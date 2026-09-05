//! Secrets — trousseau du système.
//!
//! Les secrets de l'application ne vivent **pas** dans la base SQLite locale,
//! qui est un fichier en clair lisible par n'importe quel process de la session.
//! Ils vont dans le magasin du système :
//!
//! | Plateforme | Magasin |
//! |---|---|
//! | Windows | Credential Manager |
//! | macOS | Keychain |
//! | Linux | Secret Service (D-Bus) |
//!
//! ⚠️ **Le magasin peut être absent.** Une session Linux sans bureau n'a
//! généralement pas de Secret Service, et le `keyring` échoue alors dès la
//! construction de l'entrée. C'est le prix du choix, et il est assumé : on le
//! dit clairement à l'écran plutôt que de retomber en silence sur un stockage
//! en clair — une repli discret annulerait tout le bénéfice.
//!
//! Toutes les opérations du trousseau sont **bloquantes** (appels système
//! synchrones) : elles passent par `spawn_blocking` pour ne pas figer la boucle
//! d'événements Tokio.

use anyhow::{anyhow, Context, Result};
use keyring::{Entry, Error};

/// Nom sous lequel l'application apparaît dans le trousseau.
const SERVICE: &str = "lynk-dev";

/// Clé OpenRouter. **Ne sort jamais vers le front.**
pub const OPENROUTER_KEY: &str = "openrouter-api-key";

/// Jeton du serveur MCP. Celui-ci est **relisible par le front** : l'utilisateur
/// doit pouvoir le copier dans la configuration de son client IA.
pub const MCP_TOKEN: &str = "mcp-token";

/// Message affichable quand le magasin est indisponible.
fn unavailable(err: &Error) -> anyhow::Error {
    let store = if cfg!(target_os = "windows") {
        "le Gestionnaire d'identification Windows"
    } else if cfg!(target_os = "macos") {
        "le trousseau macOS"
    } else {
        "le service de secrets (Secret Service / D-Bus)"
    };
    anyhow!("trousseau indisponible — {store} n'a pas répondu ({err})")
}

fn entry(account: &str) -> Result<Entry> {
    Entry::new(SERVICE, account).map_err(|err| unavailable(&err))
}

/// Le trousseau est-il utilisable sur cette machine ?
///
/// L'initialisation du magasin n'a lieu qu'une fois, au premier appel ; le
/// résultat est mémorisé par le `keyring`.
pub fn store_error() -> Option<String> {
    match Entry::store_status() {
        Ok(()) => None,
        Err(err) => Some(unavailable(err).to_string()),
    }
}

/// Lit un secret. `None` quand rien n'est enregistré — **pas** une erreur.
pub async fn get(account: &'static str) -> Result<Option<String>> {
    tokio::task::spawn_blocking(move || match entry(account)?.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(Error::NoEntry) => Ok(None),
        Err(err) => Err(unavailable(&err).context("lecture du trousseau")),
    })
    .await
    .context("tâche trousseau")?
}

/// Enregistre un secret, en remplaçant celui qui s'y trouve.
pub async fn set(account: &'static str, secret: &str) -> Result<()> {
    let secret = secret.to_string();
    tokio::task::spawn_blocking(move || {
        entry(account)?
            .set_password(&secret)
            .map_err(|err| unavailable(&err).context("écriture dans le trousseau"))
    })
    .await
    .context("tâche trousseau")?
}

/// Efface un secret. Effacer ce qui n'existe pas est un succès.
pub async fn delete(account: &'static str) -> Result<()> {
    tokio::task::spawn_blocking(move || match entry(account)?.delete_credential() {
        Ok(()) | Err(Error::NoEntry) => Ok(()),
        Err(err) => Err(unavailable(&err).context("effacement dans le trousseau")),
    })
    .await
    .context("tâche trousseau")?
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compte dédié aux tests : jamais celui de l'application, sinon un
    /// `cargo test` effacerait la vraie clé de la machine de développement.
    const TEST_ACCOUNT: &str = "test-round-trip";

    /// ⚠️ Ce test **écrit réellement** dans le trousseau de la machine.
    ///
    /// Il se saute là où il n'y en a pas d'utilisable — l'agent Linux de la CI
    /// n'a ni bureau ni Secret Service, et un trousseau macOS verrouillé refuse
    /// l'accès. Deux façons de le constater, et il faut les deux : le magasin
    /// peut s'initialiser (`store_error()` vide) puis refuser l'écriture.
    ///
    /// Ce que le saut ne masque pas : une écriture qui **réussit** mais range le
    /// secret au mauvais endroit se voit tout de suite, la relecture rend `None`
    /// et le test échoue.
    #[tokio::test]
    async fn round_trip_when_a_store_exists() {
        if let Some(reason) = store_error() {
            eprintln!("trousseau absent — test sauté ({reason})");
            return;
        }

        delete(TEST_ACCOUNT).await.expect("nettoyage initial");
        assert_eq!(get(TEST_ACCOUNT).await.expect("lecture vide"), None);

        if let Err(err) = set(TEST_ACCOUNT, "sk-secret").await {
            eprintln!("trousseau inaccessible — test sauté ({err:#})");
            return;
        }
        assert_eq!(
            get(TEST_ACCOUNT).await.expect("relecture"),
            Some("sk-secret".to_string())
        );

        // Un second enregistrement remplace, il n'empile pas.
        set(TEST_ACCOUNT, "sk-autre").await.expect("remplacement");
        assert_eq!(
            get(TEST_ACCOUNT).await.expect("relecture"),
            Some("sk-autre".to_string())
        );

        delete(TEST_ACCOUNT).await.expect("effacement");
        assert_eq!(get(TEST_ACCOUNT).await.expect("après effacement"), None);

        // Effacer deux fois n'est pas une erreur.
        delete(TEST_ACCOUNT).await.expect("effacement idempotent");
    }
}
