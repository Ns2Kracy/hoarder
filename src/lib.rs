#![recursion_limit = "4096"]

pub mod api;
pub mod assets;
pub mod cli;
pub mod config;
pub mod connectors;
pub mod core;
pub mod db;
pub mod entity;
pub mod error;
pub mod middleware;
pub mod server;
pub mod sync;

pub use config::AppConfig;
pub use error::{AppError, AppResult};
