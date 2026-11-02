pub mod cli;
pub mod create;
pub mod delete;
pub mod discovery;
pub mod processes;
pub mod registry;
pub mod root;
pub mod technology_menu;
pub mod tui;
pub mod update;
pub mod workspace;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
