pub mod adapters;
pub mod db;
pub mod executor_service;
pub mod handlers;
pub mod models;
pub mod scheduler;
pub mod task_manager;

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../frontend/dist/"]
pub struct Assets;
