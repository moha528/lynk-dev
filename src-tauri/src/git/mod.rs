//! Git Manager — opérations sur plusieurs dépôts.
//!
//! Découpage :
//! - [`types`] — le contrat partagé avec le front (camelCase) ;
//! - [`parse`] — les analyseurs de sortie `git`, **purs et testés** ;
//! - [`repo`] — les opérations, via le binaire `git` ;
//! - [`scan`] — la recherche de dépôts sous une racine ;
//! - [`shell`] — l'ouverture d'un terminal.
//!
//! Comme pour le Dev Manager, rien ici ne connaît Tauri : le pont IPC vit dans
//! `crate::commands::git`.

pub mod parse;
pub mod repo;
pub mod scan;
pub mod shell;
pub mod types;
