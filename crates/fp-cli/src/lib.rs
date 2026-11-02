pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub use fp_workspace::{create, discovery, processes, registry, root};
pub mod delete {
    pub use crate::delete_ui::run;
    pub use fp_workspace::delete::*;
}
pub mod workspace {
    pub use crate::workspace_ui::run;
    pub use fp_workspace::workspace::*;
}
pub mod cli;
pub mod database;
mod delete_ui;
pub mod technology_menu;
pub mod tui;
pub mod update;
mod workspace_ui;
