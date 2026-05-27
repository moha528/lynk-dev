//! Tauri command handlers exposed to the frontend via `invoke`.
//!
//! One sub-module per domain; each re-exports its `#[tauri::command]` fns.

pub mod settings;
pub mod vault;
