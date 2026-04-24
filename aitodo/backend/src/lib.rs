pub mod adapters;
pub mod db;
pub mod handlers;
pub mod models;

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../frontend/dist/"]
pub struct Assets;
