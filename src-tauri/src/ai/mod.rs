//! Assistance par modèle de langage, via OpenRouter.
//!
//! Trois usages, tous **déclenchés à la main** : rédiger un message de commit,
//! expliquer un diff, résumer une sortie de service. Rien n'est envoyé sans un
//! geste explicite de l'utilisateur — c'est la contrainte qui gouverne le
//! module, et la raison pour laquelle il n'y a aucun appel automatique.
//!
//! - [`openrouter`] — le client HTTP et le catalogue de modèles ;
//! - [`prompts`] — les consignes, pures et testables.
//!
//! Comme les autres modules métier, celui-ci ne connaît pas Tauri : le pont IPC
//! vit dans `crate::commands::ai`.

pub mod openrouter;
pub mod prompts;
