//! Núcleo síncrono sem terminal interativo: configuração, recursos, Docker e SQLite.
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
mod manager;
mod model;
mod process;
pub use manager::{Manager, valid_name};
pub use model::{Engine, Paths, Resource};

pub mod runtime;

mod sqlite;
