//! Serveur MCP — l'accès des modèles aux services locaux.
//!
//! Découpage :
//! - [`protocol`] — l'enveloppe JSON-RPC et la négociation, **pures** ;
//! - [`tools`] — le catalogue et son exécution, qui ne fait qu'appeler le
//!   superviseur du Dev Manager ;
//! - [`journal`] — la trace lisible des appels ;
//! - [`server`] — le transport HTTP et ses garde-fous.
//!
//! Comme `dev/`, **rien ici ne dépend de Tauri** : le pont IPC vit dans
//! `crate::commands::mcp`.

pub mod journal;
pub mod protocol;
pub mod server;
pub mod tools;

pub use journal::{CallRecord, Journal};
pub use server::McpServer;
pub use tools::ToolContext;

use rand_core::{OsRng, RngCore};

/// Port d'écoute par défaut.
///
/// Choisi dans la plage dynamique (49152-65535), là où aucun service standard
/// ne s'installe : le premier démarrage ne doit pas entrer en conflit avec un
/// service du profil de l'utilisateur.
pub const DEFAULT_PORT: u16 = 52780;

/// Jeton d'accès : 32 octets tirés du générateur du système, en hexadécimal.
///
/// ⚠️ `OsRng` et pas un générateur ordinaire — ce jeton est la **seule** chose
/// qui sépare un modèle autorisé d'un process quelconque de la session.
pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_is_long_and_never_twice_the_same() {
        let first = generate_token();
        assert_eq!(first.len(), 64, "32 octets en hexadécimal");
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(first, generate_token());
    }
}
