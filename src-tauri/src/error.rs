//! Error type bridged across the Tauri IPC boundary.
//!
//! `anyhow::Error` is convenient inside the backend but does not serialize
//! out of the box, so we wrap it in a transparent type that produces a
//! string payload for the frontend.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct AppError(#[from] pub anyhow::Error);

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{:#}", self.0))
    }
}
