pub mod adapters;
pub mod cli;
pub mod config;
pub mod daemon;
pub mod db;
pub mod executor_service;
pub mod handlers;
pub mod models;
pub mod scheduler;
pub mod task_manager;
pub mod todo_progress;
pub mod tunnel;

use rust_embed::RustEmbed;

#[cfg(cross_build)]
mod assets {
    use super::RustEmbed;
    #[derive(RustEmbed)]
    #[folder = "/project/frontend/dist"]
    pub struct Assets;
}

#[cfg(not(cross_build))]
mod assets {
    use super::RustEmbed;
    #[derive(RustEmbed)]
    #[folder = "../frontend/dist"]
    pub struct Assets;
}

pub use assets::Assets;
