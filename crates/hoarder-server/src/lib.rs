#![deny(warnings)]
#![recursion_limit = "4096"]

pub mod api;
pub mod app;
pub mod assets;
pub mod config;
pub mod db;
pub mod entity;
pub mod logging;
pub mod middleware;
pub mod server;

pub use config::AppConfig;
pub use hoarder_core::{AppError, AppResult};
