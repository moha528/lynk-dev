//! Dev Manager — supervision des services de développement locaux.
//!
//! Découpage :
//! - [`types`] — le contrat partagé avec le front (camelCase) ;
//! - [`supervisor`] — le cœur : démarrage, arrêt, sondes, redémarrage auto ;
//! - [`batch`] — démarrages groupés ordonnés par dépendances ;
//! - [`detect`] — la reconnaissance d'un dossier (le catalogue) ;
//! - [`scan`] — le parcours d'une arborescence ;
//! - [`net`] / [`docker`] — les sondes ;
//! - [`topo`] — le tri en couches.
//!
//! Aucun de ces modules ne dépend de Tauri : le pont IPC vit dans
//! `crate::commands::dev`, et rien d'autre ne doit s'y ajouter.

pub mod batch;
pub mod detect;
#[cfg(test)]
mod detect_tests;
pub mod docker;
pub mod net;
pub mod scan;
pub mod supervisor;
pub mod topo;
pub mod types;

pub use supervisor::{StartOptions, Supervisor};
