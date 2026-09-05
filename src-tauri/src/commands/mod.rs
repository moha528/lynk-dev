//! Tauri command handlers exposed to the frontend via `invoke`.
//!
//! One sub-module per domain; each re-exports its `#[tauri::command]` fns.

pub mod ai;
pub mod dev;
pub mod git;
pub mod settings;
pub mod vault;
