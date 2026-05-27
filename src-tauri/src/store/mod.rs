//! Persistence layer: SQLite pool + migrations + DAOs.

mod db;
pub mod settings;

pub use db::{default_db_path, init_pool, DbPool};
