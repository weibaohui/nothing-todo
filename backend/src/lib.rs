pub mod adapters;
pub mod cli;
pub mod config;
pub mod db;
pub mod executor_service;
pub mod handlers;
pub mod models;
pub mod scheduler;
pub mod task_manager;
pub mod tunnel;

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../frontend/dist/"]
pub struct Assets;
